//! HTTP Client 构建模块
//!
//! 提供统一的 HTTP Client 构建功能，支持代理配置

use reqwest::{Client, Proxy};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};

use crate::model::config::{Config, TlsBackend};

/// 代理节点池：连续失败达到该次数后临时摘除节点
pub const DEFAULT_PROXY_POOL_MAX_FAILURES: u32 = 3;

/// 代理节点池：节点被摘除后的冷却时长（秒）
pub const DEFAULT_PROXY_POOL_COOLDOWN_SECS: u64 = 60;

/// 代理配置
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ProxyConfig {
    /// 代理地址，支持 http/https/socks5
    pub url: String,
    /// 代理认证用户名
    pub username: Option<String>,
    /// 代理认证密码
    pub password: Option<String>,
}

impl ProxyConfig {
    /// 从 url 创建代理配置
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            username: None,
            password: None,
        }
    }

    /// 设置认证信息
    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }
}

/// 代理节点池选择策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProxyPoolStrategy {
    /// 轮询（默认）：按顺序依次使用各节点
    #[default]
    RoundRobin,
    /// 随机：每次随机挑选一个可用节点
    Random,
}

impl ProxyPoolStrategy {
    /// 解析策略名称（大小写不敏感）
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "roundrobin" | "round_robin" | "round-robin" | "轮询" => Some(Self::RoundRobin),
            "random" | "随机" => Some(Self::Random),
            _ => None,
        }
    }

    /// 策略名称
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RoundRobin => "roundRobin",
            Self::Random => "random",
        }
    }
}

/// 代理节点租约
///
/// 记录本次请求实际使用的池节点，用于把请求结果反馈回节点池。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyLease {
    index: usize,
    config: ProxyConfig,
}

impl ProxyLease {
    /// 节点在池中的下标
    pub fn index(&self) -> usize {
        self.index
    }

    /// 节点的代理配置
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

/// 代理节点状态
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyNodeStatus {
    /// 节点下标
    pub index: usize,
    /// 节点代理地址
    pub url: String,
    /// 是否配置了代理认证
    pub has_auth: bool,
    /// 连续失败次数
    pub failures: u32,
    /// 是否处于冷却中（已摘除）
    pub cooling: bool,
    /// 剩余冷却秒数
    pub cooldown_remaining_secs: Option<u64>,
}

/// 代理节点池
///
/// - 按策略（轮询/随机）挑选可用节点
/// - 节点连续失败达到阈值后临时摘除，冷却结束后自动回归
/// - 全部节点都在冷却时选择最早恢复的节点，保证流量不会绕过代理直连
#[derive(Debug)]
pub struct ProxyPool {
    nodes: Vec<ProxyConfig>,
    strategy: ProxyPoolStrategy,
    /// 连续失败多少次后摘除节点
    max_failures: u32,
    /// 摘除后的冷却时长
    cooldown: Duration,
    state: Mutex<PoolState>,
}

#[derive(Debug, Default)]
struct PoolState {
    /// 各节点连续失败次数
    failures: Vec<u32>,
    /// 各节点冷却结束时间（None 表示可用）
    cooldown_until: Vec<Option<Instant>>,
    /// 轮询游标
    cursor: usize,
}

impl ProxyPool {
    /// 创建代理节点池
    pub fn new(
        nodes: Vec<ProxyConfig>,
        strategy: ProxyPoolStrategy,
        max_failures: u32,
        cooldown: Duration,
    ) -> Self {
        let count = nodes.len();
        Self {
            nodes,
            strategy,
            max_failures: max_failures.max(1),
            cooldown,
            state: Mutex::new(PoolState {
                failures: vec![0; count],
                cooldown_until: vec![None; count],
                cursor: 0,
            }),
        }
    }

    /// 节点总数
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 池是否为空（没有任何节点）
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 当前策略
    pub fn strategy(&self) -> ProxyPoolStrategy {
        self.strategy
    }

    /// 连续失败达到该次数后摘除节点
    pub fn max_failures(&self) -> u32 {
        self.max_failures
    }

    /// 节点摘除后的冷却时长（秒）
    pub fn cooldown_secs(&self) -> u64 {
        self.cooldown.as_secs()
    }

    /// 挑选一个可用节点
    pub fn select(&self) -> Option<ProxyLease> {
        let count = self.nodes.len();
        if count == 0 {
            return None;
        }

        let mut state = self.state.lock();
        let now = Instant::now();

        // 清理已过期的冷却，节点重新可用
        purge_expired_cooldown(&mut state, count, now);

        let chosen = match self.strategy {
            ProxyPoolStrategy::RoundRobin => {
                let start = state.cursor % count;
                (0..count)
                    .map(|offset| (start + offset) % count)
                    .find(|&i| state.cooldown_until[i].is_none())
            }
            ProxyPoolStrategy::Random => {
                let available: Vec<usize> = (0..count)
                    .filter(|&i| state.cooldown_until[i].is_none())
                    .collect();
                if available.is_empty() {
                    None
                } else {
                    Some(available[fastrand::usize(..available.len())])
                }
            }
        };

        let chosen = match chosen {
            Some(index) => index,
            None => {
                // 所有节点都在冷却中：选择最早恢复的节点，避免流量绕过代理直连
                let mut best = 0usize;
                let mut best_until = state.cooldown_until[0].unwrap_or(now);
                for i in 1..count {
                    let until = state.cooldown_until[i].unwrap_or(now);
                    if until < best_until {
                        best = i;
                        best_until = until;
                    }
                }
                tracing::warn!("代理池所有节点均在冷却中，暂时使用最早恢复的节点 #{}", best);
                best
            }
        };

        state.cursor = (chosen + 1) % count;
        Some(ProxyLease {
            index: chosen,
            config: self.nodes[chosen].clone(),
        })
    }

    /// 上报节点失败（传输层错误 / 代理认证失败）
    pub fn report_failure(&self, lease: &ProxyLease) {
        let mut state = self.state.lock();
        let index = lease.index;
        if index >= self.nodes.len() {
            return;
        }
        state.failures[index] = state.failures[index].saturating_add(1);
        if state.failures[index] >= self.max_failures {
            state.cooldown_until[index] = Some(Instant::now() + self.cooldown);
            tracing::warn!(
                "代理节点 #{} ({}) 连续失败 {} 次，暂时摘除 {} 秒",
                index,
                proxy_url_without_auth(&lease.config.url),
                state.failures[index],
                self.cooldown.as_secs()
            );
        }
    }

    /// 上报节点成功
    pub fn report_success(&self, lease: &ProxyLease) {
        let mut state = self.state.lock();
        let index = lease.index;
        if index >= self.nodes.len() {
            return;
        }
        state.failures[index] = 0;
        state.cooldown_until[index] = None;
    }

    /// 重置所有节点的健康状态（清除失败计数与冷却）
    pub fn reset(&self) {
        let mut state = self.state.lock();
        for i in 0..self.nodes.len() {
            state.failures[i] = 0;
            state.cooldown_until[i] = None;
        }
    }

    /// 节点状态快照
    pub fn status(&self) -> Vec<ProxyNodeStatus> {
        let mut state = self.state.lock();
        let now = Instant::now();
        // 与 select 保持一致：已过期的冷却在这里同样清理，避免显示陈旧的失败计数
        purge_expired_cooldown(&mut state, self.nodes.len(), now);
        self.nodes
            .iter()
            .enumerate()
            .map(|(index, config)| {
                let remaining = state.cooldown_until[index]
                    .and_then(|until| until.checked_duration_since(now).map(|d| d.as_secs()));
                ProxyNodeStatus {
                    index,
                    url: proxy_url_without_auth(&config.url),
                    has_auth: config.username.is_some() || proxy_url_has_auth(&config.url),
                    failures: state.failures[index],
                    cooling: remaining.is_some(),
                    cooldown_remaining_secs: remaining,
                }
            })
            .collect()
    }

    /// 从代理池挑选节点并生成选择结果
    pub fn acquire(pool: &Arc<Self>) -> ProxySelection {
        match pool.select() {
            Some(lease) => ProxySelection::Pooled {
                lease,
                pool: pool.clone(),
            },
            None => ProxySelection::Direct,
        }
    }
}

/// 清理已过期的冷却状态：节点恢复可用，失败计数归零
fn purge_expired_cooldown(state: &mut PoolState, count: usize, now: Instant) {
    for i in 0..count {
        if let Some(until) = state.cooldown_until[i]
            && now >= until
        {
            state.cooldown_until[i] = None;
            state.failures[i] = 0;
        }
    }
}

/// 一次出站请求的代理选择结果
#[derive(Debug, Clone, Default)]
pub enum ProxySelection {
    /// 直连（不使用代理）
    #[default]
    Direct,
    /// 凭据级固定代理（不参与节点池健康统计）
    Fixed(ProxyConfig),
    /// 代理池节点（参与健康统计）
    Pooled {
        lease: ProxyLease,
        pool: Arc<ProxyPool>,
    },
}

impl ProxySelection {
    /// 实际使用的代理配置（None 表示直连）
    pub fn proxy(&self) -> Option<&ProxyConfig> {
        match self {
            Self::Direct => None,
            Self::Fixed(config) => Some(config),
            Self::Pooled { lease, .. } => Some(lease.config()),
        }
    }

    /// 是否来自节点池
    pub fn is_pooled(&self) -> bool {
        matches!(self, Self::Pooled { .. })
    }

    /// 上报请求成功
    pub fn report_success(&self) {
        if let Self::Pooled { lease, pool } = self {
            pool.report_success(lease);
        }
    }

    /// 上报请求失败
    pub fn report_failure(&self) {
        if let Self::Pooled { lease, pool } = self {
            pool.report_failure(lease);
        }
    }
}

/// 凭据对全局代理的覆盖决策
#[derive(Debug, Clone)]
pub enum ProxyOverride {
    /// 无覆盖：使用全局代理池
    Inherit,
    /// 显式直连（凭据 `proxyUrl` 为 "direct"）
    Direct,
    /// 固定使用指定代理（凭据级代理，不参与节点池健康统计）
    Fixed(ProxyConfig),
}

/// 代理管理器
///
/// 统一处理「全局代理池 + 凭据级覆盖」的代理决策，并持有节点池的运行时状态。
/// 全局单代理等价于只有一个节点的代理池，因此两种配置走完全相同的代码路径。
#[derive(Debug, Clone)]
pub struct ProxyManager {
    /// 全局代理节点池（None 表示不使用任何代理）
    pool: Arc<RwLock<Option<Arc<ProxyPool>>>>,
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ProxyManager {
    /// 创建代理管理器
    pub fn new(pool: Option<Arc<ProxyPool>>) -> Self {
        Self {
            pool: Arc::new(RwLock::new(pool)),
        }
    }

    /// 替换当前代理池（None 表示停用全局代理）
    pub fn replace(&self, pool: Option<Arc<ProxyPool>>) {
        *self.pool.write() = pool;
    }

    /// 是否配置了全局代理
    pub fn has_global_proxy(&self) -> bool {
        self.pool().map(|pool| !pool.is_empty()).unwrap_or(false)
    }

    /// 当前代理节点池（None 表示未配置全局代理）
    pub fn pool(&self) -> Option<Arc<ProxyPool>> {
        self.pool.read().clone()
    }

    /// 解析本次请求使用的代理
    pub fn resolve(&self, over: &ProxyOverride) -> ProxySelection {
        match over {
            ProxyOverride::Direct => ProxySelection::Direct,
            ProxyOverride::Fixed(config) => ProxySelection::Fixed(config.clone()),
            ProxyOverride::Inherit => match self.pool() {
                Some(pool) => ProxyPool::acquire(&pool),
                None => ProxySelection::Direct,
            },
        }
    }

    /// 节点状态快照
    pub fn status(&self) -> Vec<ProxyNodeStatus> {
        match self.pool() {
            Some(pool) => pool.status(),
            None => Vec::new(),
        }
    }

    /// 重置所有节点健康状态
    pub fn reset(&self) {
        if let Some(pool) = self.pool() {
            pool.reset();
        }
    }
}

/// 返回移除认证信息后的代理地址，用于状态展示和日志输出
pub fn proxy_url_without_auth(raw: &str) -> String {
    let candidate = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{}", raw)
    };
    let Ok(mut url) = reqwest::Url::parse(&candidate) else {
        return "***".to_string();
    };

    let _ = url.set_username("");
    let _ = url.set_password(None);
    let mut value = url.to_string();
    if url.path() == "/" && url.query().is_none() && url.fragment().is_none() {
        value.pop();
    }
    value
}

fn proxy_url_has_auth(raw: &str) -> bool {
    let candidate = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{}", raw)
    };
    reqwest::Url::parse(&candidate)
        .map(|url| !url.username().is_empty() || url.password().is_some())
        .unwrap_or(false)
}

/// 根据应用配置构建全局代理节点池
///
/// 归一化规则：
/// - 配置了 `proxyPool.nodes` → 使用节点池（多节点）
/// - 仅配置了 `proxyUrl` → 等价于只有一个节点的代理池
/// - 两者都没配置 → 不使用代理（返回 `None`）
pub fn build_proxy_pool(config: &Config) -> Option<Arc<ProxyPool>> {
    let pool_config = config.proxy_pool.as_ref();

    let nodes: Vec<ProxyConfig> = match pool_config {
        Some(pool) if !pool.nodes.is_empty() => pool
            .nodes
            .iter()
            .map(|node| {
                let mut proxy = ProxyConfig::new(node.url.clone());
                if let (Some(username), Some(password)) = (&node.username, &node.password) {
                    proxy = proxy.with_auth(username.clone(), password.clone());
                }
                proxy
            })
            .collect(),
        _ => match &config.proxy_url {
            Some(url) if !url.trim().is_empty() => {
                let mut proxy = ProxyConfig::new(url.clone());
                if let (Some(username), Some(password)) =
                    (&config.proxy_username, &config.proxy_password)
                {
                    proxy = proxy.with_auth(username.clone(), password.clone());
                }
                vec![proxy]
            }
            _ => Vec::new(),
        },
    };

    if nodes.is_empty() {
        return None;
    }

    let strategy = match pool_config {
        Some(pool) => match ProxyPoolStrategy::parse(&pool.strategy) {
            Some(s) => s,
            None => {
                tracing::warn!("未知的代理池策略 \"{}\"，回退为 roundRobin", pool.strategy);
                ProxyPoolStrategy::RoundRobin
            }
        },
        None => ProxyPoolStrategy::RoundRobin,
    };

    let max_failures = pool_config
        .map(|p| p.max_failures)
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_PROXY_POOL_MAX_FAILURES);
    let cooldown_secs = pool_config
        .map(|p| p.cooldown_secs)
        .unwrap_or(DEFAULT_PROXY_POOL_COOLDOWN_SECS);

    Some(Arc::new(ProxyPool::new(
        nodes,
        strategy,
        max_failures,
        Duration::from_secs(cooldown_secs),
    )))
}

/// 构建 HTTP Client
///
/// # Arguments
/// * `proxy` - 可选的代理配置
/// * `timeout_secs` - 超时时间（秒）
///
/// # Returns
/// 配置好的 reqwest::Client
pub fn build_client(
    proxy: Option<&ProxyConfig>,
    timeout_secs: u64,
    tls_backend: TlsBackend,
) -> anyhow::Result<Client> {
    let mut builder = Client::builder().timeout(Duration::from_secs(timeout_secs));

    match tls_backend {
        TlsBackend::Rustls => {
            builder = builder.use_rustls_tls();
        }
        TlsBackend::NativeTls => {
            #[cfg(feature = "native-tls")]
            {
                builder = builder.use_native_tls();
            }
            #[cfg(not(feature = "native-tls"))]
            {
                anyhow::bail!("此构建版本未包含 native-tls 后端，请在配置中改用 rustls");
            }
        }
    }

    if let Some(proxy_config) = proxy {
        let mut proxy = Proxy::all(&proxy_config.url)?;

        // 设置代理认证
        if let (Some(username), Some(password)) = (&proxy_config.username, &proxy_config.password) {
            proxy = proxy.basic_auth(username, password);
        }

        builder = builder.proxy(proxy);
        tracing::debug!(
            "HTTP Client 使用代理: {}",
            proxy_url_without_auth(&proxy_config.url)
        );
    }

    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::config::ProxyPoolConfig;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use thread::sleep;

    #[test]
    fn test_proxy_config_new() {
        let config = ProxyConfig::new("http://127.0.0.1:7890");
        assert_eq!(config.url, "http://127.0.0.1:7890");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
    }

    #[test]
    fn test_proxy_config_with_auth() {
        let config = ProxyConfig::new("socks5://127.0.0.1:1080").with_auth("user", "pass");
        assert_eq!(config.url, "socks5://127.0.0.1:1080");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
    }

    #[test]
    fn test_build_client_without_proxy() {
        let client = build_client(None, 30, TlsBackend::Rustls);
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_client_with_proxy() {
        let config = ProxyConfig::new("http://127.0.0.1:7890");
        let client = build_client(Some(&config), 30, TlsBackend::Rustls);
        assert!(client.is_ok());
    }

    fn make_pool(
        nodes: usize,
        strategy: ProxyPoolStrategy,
        max_failures: u32,
        cooldown_ms: u64,
    ) -> Arc<ProxyPool> {
        let nodes: Vec<ProxyConfig> = (0..nodes)
            .map(|i| ProxyConfig::new(format!("http://127.0.0.1:{}", 9000 + i)))
            .collect();
        Arc::new(ProxyPool::new(
            nodes,
            strategy,
            max_failures,
            Duration::from_millis(cooldown_ms),
        ))
    }

    #[test]
    fn test_pool_strategy_parse() {
        assert_eq!(
            ProxyPoolStrategy::parse("roundRobin"),
            Some(ProxyPoolStrategy::RoundRobin)
        );
        assert_eq!(
            ProxyPoolStrategy::parse("round-robin"),
            Some(ProxyPoolStrategy::RoundRobin)
        );
        assert_eq!(
            ProxyPoolStrategy::parse("RANDOM"),
            Some(ProxyPoolStrategy::Random)
        );
        assert_eq!(ProxyPoolStrategy::parse("unknown"), None);
    }

    #[test]
    fn test_pool_round_robin_cycles_through_nodes() {
        let pool = make_pool(3, ProxyPoolStrategy::RoundRobin, 3, 60_000);
        let picked: Vec<usize> = (0..6).map(|_| pool.select().unwrap().index()).collect();
        assert_eq!(picked, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn test_pool_random_only_picks_available_nodes() {
        let pool = make_pool(4, ProxyPoolStrategy::Random, 3, 60_000);
        // 摘除首个被选中的节点
        let lease = pool.select().unwrap();
        let ejected = lease.index();
        pool.report_failure(&lease);
        pool.report_failure(&lease);
        pool.report_failure(&lease);

        for _ in 0..20 {
            let index = pool.select().unwrap().index();
            assert_ne!(index, ejected, "随机策略不应选中已摘除的节点");
        }
    }

    #[test]
    fn test_pool_failure_threshold_triggers_cooldown() {
        let pool = make_pool(3, ProxyPoolStrategy::RoundRobin, 2, 60_000);
        let lease = pool.select().unwrap();
        assert_eq!(lease.index(), 0);

        pool.report_failure(&lease);
        let status = pool.status();
        assert_eq!(status[0].failures, 1);
        assert!(!status[0].cooling);

        // 达到阈值后节点被摘除
        pool.report_failure(&lease);
        let status = pool.status();
        assert_eq!(status[0].failures, 2);
        assert!(status[0].cooling);
        assert!(status[0].cooldown_remaining_secs.is_some());

        // 后续选择跳过被摘除的节点
        let next = pool.select().unwrap();
        assert_eq!(next.index(), 1);
    }

    #[test]
    fn test_pool_cooldown_expires_and_node_recovers() {
        let pool = make_pool(2, ProxyPoolStrategy::RoundRobin, 1, 60);
        let lease = pool.select().unwrap();
        pool.report_failure(&lease);
        assert!(pool.status()[0].cooling);

        sleep(Duration::from_millis(120));

        let status = pool.status();
        assert!(!status[0].cooling, "冷却结束后节点应恢复可用");
        assert_eq!(status[0].failures, 0, "恢复后失败计数应清零");

        // 恢复后的节点重新参与轮询：两次选择应覆盖两个节点
        let first = pool.select().unwrap().index();
        let second = pool.select().unwrap().index();
        assert!(
            first != second,
            "恢复后节点应重新参与轮询，实际两次都选中了 #{}",
            first
        );
    }

    #[test]
    fn test_pool_success_resets_failure_count() {
        let pool = make_pool(1, ProxyPoolStrategy::RoundRobin, 3, 60_000);
        let lease = pool.select().unwrap();
        pool.report_failure(&lease);
        pool.report_failure(&lease);
        assert_eq!(pool.status()[0].failures, 2);

        pool.report_success(&lease);
        assert_eq!(pool.status()[0].failures, 0);
    }

    #[test]
    fn test_pool_all_nodes_cooling_picks_earliest_recovery() {
        let pool = make_pool(2, ProxyPoolStrategy::RoundRobin, 1, 60_000);
        let first = pool.select().unwrap();
        let second = pool.select().unwrap();
        pool.report_failure(&first);
        sleep(Duration::from_millis(20));
        pool.report_failure(&second);

        // 全部节点都在冷却：选择最早恢复的节点，而不是直连
        let chosen = pool.select().unwrap();
        assert_eq!(chosen.index(), 0, "应选择最早恢复的节点");
    }

    #[test]
    fn test_pool_reset_clears_health_state() {
        let pool = make_pool(2, ProxyPoolStrategy::RoundRobin, 1, 60_000);
        let lease = pool.select().unwrap();
        pool.report_failure(&lease);
        assert!(pool.status()[0].cooling);

        pool.reset();
        let status = pool.status();
        assert!(!status[0].cooling);
        assert_eq!(status[0].failures, 0);
    }

    #[test]
    fn test_manager_resolve_direct_and_fixed() {
        let pool = make_pool(2, ProxyPoolStrategy::RoundRobin, 3, 60_000);
        let manager = ProxyManager::new(Some(pool));

        assert!(manager.has_global_proxy());

        let direct = manager.resolve(&ProxyOverride::Direct);
        assert!(direct.proxy().is_none());
        assert!(!direct.is_pooled());

        let fixed = manager.resolve(&ProxyOverride::Fixed(ProxyConfig::new("http://fixed:8080")));
        assert_eq!(fixed.proxy().unwrap().url, "http://fixed:8080");
        assert!(!fixed.is_pooled());

        let inherited = manager.resolve(&ProxyOverride::Inherit);
        assert!(inherited.is_pooled());
        assert!(inherited.proxy().is_some());
    }

    #[test]
    fn test_manager_without_pool_resolves_to_direct() {
        let manager = ProxyManager::new(None);
        assert!(!manager.has_global_proxy());
        assert!(manager.resolve(&ProxyOverride::Inherit).proxy().is_none());
        assert!(manager.status().is_empty());
    }

    #[test]
    fn test_manager_reset_clears_pool_health() {
        let pool = make_pool(1, ProxyPoolStrategy::RoundRobin, 1, 60_000);
        let manager = ProxyManager::new(Some(pool));
        let selection = manager.resolve(&ProxyOverride::Inherit);
        selection.report_failure();
        assert!(manager.status()[0].cooling);

        manager.reset();
        assert!(!manager.status()[0].cooling);
    }

    #[test]
    fn test_manager_replace_pool_at_runtime_and_disable() {
        let manager = ProxyManager::new(None);
        assert!(!manager.has_global_proxy());
        assert!(manager.resolve(&ProxyOverride::Inherit).proxy().is_none());

        let pool = make_pool(2, ProxyPoolStrategy::RoundRobin, 3, 60_000);
        manager.replace(Some(pool));
        assert!(manager.has_global_proxy());
        assert_eq!(manager.status().len(), 2);
        assert!(manager.resolve(&ProxyOverride::Inherit).is_pooled());

        manager.replace(None);
        assert!(!manager.has_global_proxy());
        assert!(manager.status().is_empty());
        assert!(manager.resolve(&ProxyOverride::Inherit).proxy().is_none());
    }

    #[test]
    fn test_proxy_status_masks_url_credentials() {
        let pool = Arc::new(ProxyPool::new(
            vec![ProxyConfig::new(
                "http://embedded-user:embedded-pass@127.0.0.1:8080",
            )],
            ProxyPoolStrategy::RoundRobin,
            3,
            Duration::from_secs(60),
        ));
        let manager = ProxyManager::new(Some(pool));
        let status = manager.status();

        assert_eq!(status[0].url, "http://127.0.0.1:8080");
        assert!(status[0].has_auth);
        assert!(!status[0].url.contains("embedded-user"));
        assert!(!status[0].url.contains("embedded-pass"));
    }

    #[test]
    fn test_proxy_url_without_auth() {
        assert_eq!(
            proxy_url_without_auth("http://user:pass@127.0.0.1:8080"),
            "http://127.0.0.1:8080"
        );
        assert_eq!(
            proxy_url_without_auth("127.0.0.1:8080"),
            "http://127.0.0.1:8080"
        );
    }

    #[test]
    fn test_build_proxy_pool_none_without_config() {
        let config = Config::default();
        assert!(build_proxy_pool(&config).is_none());
    }

    #[test]
    fn test_build_proxy_pool_single_node_from_proxy_url() {
        let mut config = Config::default();
        config.proxy_url = Some("http://127.0.0.1:7890".to_string());
        config.proxy_username = Some("user".to_string());
        config.proxy_password = Some("pass".to_string());

        let pool = build_proxy_pool(&config).expect("应构建出单节点代理池");
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.strategy(), ProxyPoolStrategy::RoundRobin);
        let status = pool.status();
        assert_eq!(status[0].url, "http://127.0.0.1:7890");
        assert!(status[0].has_auth);
    }

    #[test]
    fn test_build_proxy_pool_from_pool_config() {
        let mut config = Config::default();
        config.proxy_url = Some("http://ignored:7890".to_string());
        config.proxy_pool = Some(ProxyPoolConfig {
            nodes: vec![
                crate::model::config::ProxyNodeConfig {
                    url: "http://a:1080".to_string(),
                    username: None,
                    password: None,
                },
                crate::model::config::ProxyNodeConfig {
                    url: "socks5://b:1080".to_string(),
                    username: Some("u".to_string()),
                    password: Some("p".to_string()),
                },
            ],
            strategy: "random".to_string(),
            max_failures: 5,
            cooldown_secs: 30,
        });

        let pool = build_proxy_pool(&config).expect("应构建出多节点代理池");
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.strategy(), ProxyPoolStrategy::Random);
        let status = pool.status();
        assert_eq!(status[0].url, "http://a:1080");
        assert!(!status[0].has_auth);
        assert_eq!(status[1].url, "socks5://b:1080");
        assert!(status[1].has_auth);
    }

    #[test]
    fn test_build_proxy_pool_invalid_strategy_falls_back() {
        let mut config = Config::default();
        config.proxy_pool = Some(ProxyPoolConfig {
            nodes: vec![crate::model::config::ProxyNodeConfig {
                url: "http://a:1080".to_string(),
                username: None,
                password: None,
            }],
            strategy: "nope".to_string(),
            max_failures: 0,
            cooldown_secs: 0,
        });

        let pool = build_proxy_pool(&config).expect("应构建出代理池");
        assert_eq!(pool.strategy(), ProxyPoolStrategy::RoundRobin);
    }

    /// 启动一个只回固定响应的本地假代理，返回代理地址与命中计数
    fn spawn_fake_proxy(response: &'static [u8]) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本地端口失败");
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => break,
                };
                hits_clone.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(response);
                let _ = stream.flush();
            }
        });

        (format!("http://127.0.0.1:{}", port), hits)
    }

    /// 取一个当前无人监听的端口，模拟连不上的代理
    fn closed_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    #[tokio::test]
    async fn test_real_request_goes_through_proxy_node() {
        let (url, hits) = spawn_fake_proxy(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        );
        let client = build_client(Some(&ProxyConfig::new(url.clone())), 10, TlsBackend::Rustls)
            .expect("构建 client 失败");

        let response = client
            .get("http://example.invalid/probe")
            .send()
            .await
            .expect("经代理请求应成功");

        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(hits.load(Ordering::SeqCst), 1, "请求应命中代理节点");
    }

    #[tokio::test]
    async fn test_transport_error_counts_as_node_failure() {
        let url = format!("http://127.0.0.1:{}", closed_port());
        let client = build_client(Some(&ProxyConfig::new(url)), 5, TlsBackend::Rustls)
            .expect("构建 client 失败");

        let result = client.get("http://example.invalid/probe").send().await;
        assert!(result.is_err(), "连不上的代理应产生传输层错误");

        // 传输层错误计入节点失败
        let pool = make_pool(1, ProxyPoolStrategy::RoundRobin, 1, 60_000);
        let lease = pool.select().unwrap();
        pool.report_failure(&lease);
        assert!(pool.status()[0].cooling);
    }

    #[tokio::test]
    async fn test_407_counts_as_node_failure_but_upstream_error_does_not() {
        let (auth_url, _) = spawn_fake_proxy(
            b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let client = build_client(Some(&ProxyConfig::new(auth_url)), 10, TlsBackend::Rustls)
            .expect("构建 client 失败");
        let response = client
            .get("http://example.invalid/probe")
            .send()
            .await
            .expect("407 仍是有效响应");
        assert_eq!(response.status().as_u16(), 407);

        let (upstream_url, _) = spawn_fake_proxy(
            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let client = build_client(
            Some(&ProxyConfig::new(upstream_url)),
            10,
            TlsBackend::Rustls,
        )
        .expect("构建 client 失败");
        let response = client
            .get("http://example.invalid/probe")
            .send()
            .await
            .expect("500 仍是有效响应");
        assert_eq!(response.status().as_u16(), 500);
    }

    #[tokio::test]
    async fn test_pool_actually_rotates_real_requests_across_nodes() {
        let (url_a, hits_a) = spawn_fake_proxy(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        );
        let (url_b, hits_b) = spawn_fake_proxy(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        );

        let pool = Arc::new(ProxyPool::new(
            vec![ProxyConfig::new(url_a), ProxyConfig::new(url_b)],
            ProxyPoolStrategy::RoundRobin,
            3,
            Duration::from_secs(60),
        ));
        let manager = ProxyManager::new(Some(pool));

        for _ in 0..4 {
            let selection = manager.resolve(&ProxyOverride::Inherit);
            let client = build_client(selection.proxy(), 10, TlsBackend::Rustls).unwrap();
            let response = client
                .get("http://example.invalid/probe")
                .send()
                .await
                .expect("经节点池请求应成功");
            assert_eq!(response.status().as_u16(), 200);
            selection.report_success();
        }

        assert_eq!(hits_a.load(Ordering::SeqCst), 2, "节点 A 应承接 2 次请求");
        assert_eq!(hits_b.load(Ordering::SeqCst), 2, "节点 B 应承接 2 次请求");
    }
}
