import { useRef, useState } from 'react'
import {
  Globe,
  Network,
  RotateCcw,
  Save,
  Settings2,
  ShieldAlert,
  ShieldCheck,
  Upload,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  useProxyPoolStatus,
  useResetProxyPool,
  useSetProxyPool,
} from '@/hooks/use-credentials'
import { extractErrorMessage } from '@/lib/utils'
import { toast } from 'sonner'

const STRATEGY_LABELS: Record<string, string> = {
  roundRobin: '轮询',
  random: '随机',
}

type ProxyPoolStrategy = 'roundRobin' | 'random'

export function ProxyPoolCard() {
  const { data, isLoading } = useProxyPoolStatus()
  const { mutate: resetPool, isPending: isResetting } = useResetProxyPool()
  const { mutate: savePool, isPending: isSaving } = useSetProxyPool()

  const [dialogOpen, setDialogOpen] = useState(false)
  const [enabled, setEnabled] = useState(false)
  const [proxyUrls, setProxyUrls] = useState('')
  const [strategy, setStrategy] = useState<ProxyPoolStrategy>('roundRobin')
  const [maxFailures, setMaxFailures] = useState('3')
  const [cooldownSecs, setCooldownSecs] = useState('60')
  const fileInputRef = useRef<HTMLInputElement>(null)

  const openSettings = () => {
    setEnabled(data?.enabled ?? false)
    setProxyUrls(data?.proxyUrls ?? '')
    setStrategy(data?.strategy === 'random' ? 'random' : 'roundRobin')
    setMaxFailures(String(data?.maxFailures ?? 3))
    setCooldownSecs(String(data?.cooldownSecs ?? 60))
    setDialogOpen(true)
  }

  const handleReset = () => {
    resetPool(undefined, {
      onSuccess: () => {
        toast.success('代理节点池状态已重置')
      },
      onError: (error) => {
        toast.error(`重置失败: ${extractErrorMessage(error)}`)
      },
    })
  }

  const handleFile = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (!file) return

    if (!file.name.toLowerCase().endsWith('.txt')) {
      toast.error('请选择 .txt 文件')
      return
    }
    if (file.size > 2 * 1024 * 1024) {
      toast.error('代理池文件不能超过 2 MB')
      return
    }

    try {
      const content = await file.text()
      if (!content.trim()) {
        toast.error('文件中没有代理地址')
        return
      }
      setProxyUrls(content.replace(/\r\n?/g, '\n'))
      const count = content
        .split(/\r?\n/)
        .filter((line) => line.trim() && !line.trim().startsWith('#'))
        .length
      toast.success(`已读取 ${count} 个代理地址`)
    } catch {
      toast.error('读取代理池文件失败')
    }
  }

  const handleSave = (event: React.FormEvent) => {
    event.preventDefault()

    const failures = Number.parseInt(maxFailures, 10)
    const cooldown = Number.parseInt(cooldownSecs, 10)
    if (enabled && !proxyUrls.trim()) {
      toast.error('请填写至少一个代理地址')
      return
    }
    if (!Number.isInteger(failures) || failures < 1 || failures > 1000) {
      toast.error('失败阈值必须在 1 到 1000 之间')
      return
    }
    if (!Number.isInteger(cooldown) || cooldown < 1 || cooldown > 604800) {
      toast.error('冷却时间必须在 1 到 604800 秒之间')
      return
    }

    savePool(
      {
        enabled,
        proxyUrls,
        strategy,
        maxFailures: failures,
        cooldownSecs: cooldown,
      },
      {
        onSuccess: (result) => {
          toast.success(result.enabled ? '代理节点池已保存并启用' : '代理节点池已停用')
          setDialogOpen(false)
        },
        onError: (error) => {
          toast.error(`保存失败: ${extractErrorMessage(error)}`)
        },
      }
    )
  }

  if (isLoading) {
    return (
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium text-muted-foreground">
            代理节点池
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground">加载中...</div>
        </CardContent>
      </Card>
    )
  }

  if (!data?.enabled) {
    return (
      <>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <Network className="h-4 w-4" />
              代理节点池
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div className="text-sm text-muted-foreground">未启用，当前所有请求直连。</div>
              <Button variant="outline" size="sm" onClick={openSettings}>
                <Settings2 />
                配置代理池
              </Button>
            </div>
          </CardContent>
        </Card>
        {renderSettingsDialog()}
      </>
    )
  }

  return (
    <>
      <Card>
        <CardHeader className="pb-2">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <Network className="h-4 w-4" />
              代理节点池
            </CardTitle>
            <div className="flex flex-wrap gap-2">
              <Button variant="outline" size="sm" onClick={openSettings}>
                <Settings2 />
                设置
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handleReset}
                disabled={isResetting}
                title="清除节点失败计数与冷却状态"
              >
                <RotateCcw />
                重置状态
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-3 mb-4 text-sm sm:flex sm:flex-wrap sm:items-center sm:gap-4">
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground">策略</span>
              <Badge variant="secondary">
                {STRATEGY_LABELS[data.strategy] || data.strategy}
              </Badge>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground">可用节点</span>
              <span className="font-semibold text-green-600">
                {data.available}/{data.total}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground">失败阈值</span>
              <span className="font-medium">{data.maxFailures} 次</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground">冷却</span>
              <span className="font-medium">{data.cooldownSecs} 秒</span>
            </div>
          </div>

          <div className="space-y-2">
            {data.nodes.map((node) => (
              <div
                key={node.index}
                className="flex flex-col gap-2 rounded-md border px-3 py-2 text-sm sm:flex-row sm:items-center sm:justify-between"
              >
                <div className="flex min-w-0 items-center gap-2">
                  {node.cooling ? (
                    <ShieldAlert className="h-4 w-4 text-yellow-500 shrink-0" />
                  ) : (
                    <ShieldCheck className="h-4 w-4 text-green-600 shrink-0" />
                  )}
                  <Globe className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                  <span className="min-w-0 truncate font-mono text-xs" title={node.url}>
                    {node.url}
                  </span>
                  {node.hasAuth && (
                    <Badge variant="outline" className="shrink-0">
                      认证
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-2 shrink-0 pl-6 sm:pl-0">
                  {node.cooling ? (
                    <Badge variant="warning">
                      冷却 {node.cooldownRemainingSecs ?? 0}s
                    </Badge>
                  ) : (
                    <Badge variant="success">可用</Badge>
                  )}
                  {node.failures > 0 && (
                    <span className="text-xs text-muted-foreground">
                      失败 {node.failures}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
      {renderSettingsDialog()}
    </>
  )

  function renderSettingsDialog() {
    return (
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] flex flex-col">
          <DialogHeader>
            <DialogTitle>代理节点池设置</DialogTitle>
          </DialogHeader>

          <form onSubmit={handleSave} className="flex min-h-0 flex-1 flex-col">
            <div className="flex-1 space-y-5 overflow-y-auto py-4 pr-1">
              <div className="flex items-center justify-between gap-4 rounded-md border px-3 py-3">
                <div className="flex items-center gap-2 text-sm font-medium">
                  <Network className="h-4 w-4 text-muted-foreground" />
                  启用代理节点池
                </div>
                <Switch
                  checked={enabled}
                  onCheckedChange={setEnabled}
                  disabled={isSaving}
                  aria-label="启用代理节点池"
                />
              </div>

              <div className="space-y-2">
                <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                  <label htmlFor="proxyUrls" className="text-sm font-medium">
                    代理地址
                  </label>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => fileInputRef.current?.click()}
                    disabled={isSaving}
                  >
                    <Upload />
                    上传 TXT
                  </Button>
                  <input
                    ref={fileInputRef}
                    type="file"
                    accept=".txt,text/plain"
                    className="hidden"
                    onChange={handleFile}
                    disabled={isSaving}
                  />
                </div>
                <textarea
                  id="proxyUrls"
                  value={proxyUrls}
                  onChange={(event) => setProxyUrls(event.target.value)}
                  disabled={isSaving}
                  spellCheck={false}
                  placeholder={
                    'http://127.0.0.1:8080\nsocks5://user:pass@127.0.0.1:1080\n# 支持注释与无协议地址'
                  }
                  className="min-h-48 w-full resize-y rounded-md border border-input bg-background px-3 py-2 font-mono text-xs leading-6 ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                />
              </div>

              <div className="grid gap-4 sm:grid-cols-3">
                <div className="space-y-2">
                  <label htmlFor="proxyStrategy" className="text-sm font-medium">
                    选择策略
                  </label>
                  <select
                    id="proxyStrategy"
                    value={strategy}
                    onChange={(event) =>
                      setStrategy(event.target.value as ProxyPoolStrategy)
                    }
                    disabled={isSaving}
                    className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    <option value="roundRobin">轮询</option>
                    <option value="random">随机</option>
                  </select>
                </div>
                <div className="space-y-2">
                  <label htmlFor="maxFailures" className="text-sm font-medium">
                    失败阈值
                  </label>
                  <Input
                    id="maxFailures"
                    type="number"
                    min={1}
                    max={1000}
                    value={maxFailures}
                    onChange={(event) => setMaxFailures(event.target.value)}
                    disabled={isSaving}
                  />
                </div>
                <div className="space-y-2">
                  <label htmlFor="cooldownSecs" className="text-sm font-medium">
                    冷却秒数
                  </label>
                  <Input
                    id="cooldownSecs"
                    type="number"
                    min={1}
                    max={604800}
                    value={cooldownSecs}
                    onChange={(event) => setCooldownSecs(event.target.value)}
                    disabled={isSaving}
                  />
                </div>
              </div>
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setDialogOpen(false)}
                disabled={isSaving}
              >
                取消
              </Button>
              <Button type="submit" disabled={isSaving}>
                <Save />
                {isSaving ? '保存中...' : '保存'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    )
  }
}
