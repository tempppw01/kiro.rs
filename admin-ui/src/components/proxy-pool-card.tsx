import { Globe, Network, RotateCcw, ShieldCheck, ShieldAlert } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { useProxyPoolStatus, useResetProxyPool } from '@/hooks/use-credentials'
import { extractErrorMessage } from '@/lib/utils'
import { toast } from 'sonner'

const STRATEGY_LABELS: Record<string, string> = {
  roundRobin: '轮询',
  random: '随机',
}

export function ProxyPoolCard() {
  const { data, isLoading } = useProxyPoolStatus()
  const { mutate: resetPool, isPending: isResetting } = useResetProxyPool()

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
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
            <Network className="h-4 w-4" />
            代理节点池
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-sm text-muted-foreground">
            未配置代理节点池，所有请求直连。可在配置文件的 <code>proxyPool</code> 中启用。
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between gap-2">
          <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
            <Network className="h-4 w-4" />
            代理节点池
          </CardTitle>
          <Button
            variant="outline"
            size="sm"
            onClick={handleReset}
            disabled={isResetting}
            title="清除节点失败计数与冷却状态"
          >
            <RotateCcw className="h-4 w-4 mr-1" />
            重置状态
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap items-center gap-4 mb-4 text-sm">
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
        </div>

        <div className="space-y-2">
          {data.nodes.map(node => (
            <div
              key={node.index}
              className="flex items-center justify-between gap-3 rounded-md border px-3 py-2 text-sm"
            >
              <div className="flex items-center gap-2 min-w-0">
                {node.cooling ? (
                  <ShieldAlert className="h-4 w-4 text-yellow-500 shrink-0" />
                ) : (
                  <ShieldCheck className="h-4 w-4 text-green-600 shrink-0" />
                )}
                <Globe className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <span className="truncate" title={node.url}>
                  {node.url}
                </span>
                {node.hasAuth && (
                  <Badge variant="outline" className="shrink-0">
                    认证
                  </Badge>
                )}
              </div>
              <div className="flex items-center gap-2 shrink-0">
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
  )
}
