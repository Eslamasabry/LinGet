import { useEffect } from 'react'
import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event'
import type { Dispatch, SetStateAction } from 'react'
import type {
  BatchUpdateItemPayload,
  BatchUpdateProgressPayload,
  OperationLogPayload,
  OperationProgressPayload,
  RunningOperation,
} from '../types'

export function useOperationEvents({
  setRunningOperations,
  setUpdateProgress,
}: {
  setRunningOperations: Dispatch<SetStateAction<RunningOperation[]>>
  setUpdateProgress: Dispatch<SetStateAction<number>>
}) {
  useEffect(() => {
    let cleanupFns: UnlistenFn[] = []
    let active = true

    const upsertOperation = (
      current: RunningOperation[],
      operationId: string,
      updater: (existing: RunningOperation | undefined) => RunningOperation,
    ) => {
      const existing = current.find((operation) => operation.id === operationId)
      const next = updater(existing)
      if (existing) {
        return current.map((operation) => (operation.id === operationId ? next : operation))
      }
      return [...current, next]
    }

    void Promise.all([
      listen<OperationProgressPayload>('operation-progress', (event: Event<OperationProgressPayload>) => {
        const data = event.payload
        const operationId = data.operation_id || data.name

        setRunningOperations((current) =>
          upsertOperation(current, operationId, (existing) => ({
            id: operationId,
            name: data.name,
            type: operationId.includes('install') ? 'install' : operationId.includes('remove') ? 'remove' : 'update',
            status: data.status,
            progress: data.progress ?? existing?.progress ?? 0,
            message: data.message,
            logs: existing?.logs ?? [],
            startTime: existing?.startTime ?? new Date(),
          })),
        )
      }),
      listen<OperationLogPayload>('operation-log', (event: Event<OperationLogPayload>) => {
        const data = event.payload
        const operationId = data.operation_id || data.name

        setRunningOperations((current) =>
          current.map((operation) => {
            if (operation.id !== operationId) {
              return operation
            }

            return {
              ...operation,
              logs: [
                ...operation.logs,
                {
                  id: `${Date.now()}`,
                  timestamp: new Date(),
                  type: data.is_error ? 'error' : 'info',
                  message: data.line,
                  name: data.name,
                },
              ],
            }
          }),
        )
      }),
      listen<BatchUpdateProgressPayload>('batch-update-progress', (event: Event<BatchUpdateProgressPayload>) => {
        setUpdateProgress(event.payload.completed)
      }),
      listen<BatchUpdateItemPayload>('update-progress', (event: Event<BatchUpdateItemPayload>) => {
        const data = event.payload
        const operationId = `batch-update-${data.source}-${data.name}`

        setRunningOperations((current) =>
          upsertOperation(current, operationId, (existing) => ({
            id: operationId,
            name: data.name,
            type: 'update',
            status: data.status === 'updated' ? 'completed' : 'failed',
            progress: 100,
            message: data.error ?? (data.status === 'updated' ? `Updated ${data.name}` : `Failed to update ${data.name}`),
            logs: existing?.logs ?? [],
            startTime: existing?.startTime ?? new Date(),
          })),
        )
      }),
      listen('batch-update-completed', () => {
        setRunningOperations((current) =>
          current.filter(
            (operation) => !operation.id.startsWith('batch-update-') || operation.status === 'failed',
          ),
        )
      }),
    ]).then((listeners) => {
      if (active) {
        cleanupFns = listeners
      } else {
        listeners.forEach((listener) => listener())
      }
    })

    return () => {
      active = false
      cleanupFns.forEach((listener) => listener())
    }
  }, [setRunningOperations, setUpdateProgress])
}
