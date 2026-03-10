import { useEffect, useRef } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { AlertCircle, Check, Download, RotateCcw, Terminal as TerminalIcon, Trash2, X } from 'lucide-react'
import type { RunningOperation } from '../types'

export function TaskHub({
  operations,
  onCancelOperation,
}: {
  operations: RunningOperation[]
  onCancelOperation: (id: string) => void
}) {
  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [operations])

  if (operations.length === 0) {
    return (
      <div className="p-6 text-center">
        <TerminalIcon size={48} className="mx-auto text-gray-600 mb-4" />
        <h3 className="text-lg font-medium text-white mb-2">No operations running</h3>
        <p className="text-sm text-gray-500">Package operations will appear here</p>
      </div>
    )
  }

  return (
    <div className="p-4 space-y-4 max-h-96 overflow-y-auto" ref={scrollRef}>
      {operations.map((operation) => (
        <motion.div
          key={operation.id}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-4"
        >
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              {operation.type === 'install' && <Download size={16} className="text-blue-400" />}
              {operation.type === 'remove' && <Trash2 size={16} className="text-red-400" />}
              {operation.type === 'update' && <RotateCcw size={16} className="text-yellow-400" />}
              <span className="font-medium text-white">{operation.name}</span>
            </div>
            <div className="flex items-center gap-2">
              {operation.status === 'completed' && <Check size={16} className="text-green-400" />}
              {operation.status === 'failed' && <AlertCircle size={16} className="text-red-400" />}
              {operation.status !== 'completed' && operation.status !== 'failed' && (
                <button
                  onClick={() => onCancelOperation(operation.id)}
                  className="p-1 text-gray-400 hover:text-red-400 transition-colors"
                  title="Cancel"
                >
                  <X size={14} />
                </button>
              )}
            </div>
          </div>

          <div className="mb-2">
            <div className="flex items-center justify-between text-xs text-gray-500 mb-1">
              <span>{operation.message}</span>
              <span>{operation.progress}%</span>
            </div>
            <div className="h-1 bg-[#1A1A1A] rounded-full overflow-hidden">
              <motion.div
                className={`h-full ${
                  operation.status === 'failed'
                    ? 'bg-red-500'
                    : operation.status === 'completed'
                      ? 'bg-green-500'
                      : 'bg-blue-500'
                }`}
                initial={{ width: 0 }}
                animate={{ width: `${operation.progress}%` }}
              />
            </div>
          </div>

          {operation.logs.length > 0 && (
            <div className="mt-2 pt-2 border-t border-[#27272A] max-h-24 overflow-y-auto">
              {operation.logs.slice(-5).map((log, index) => (
                <div key={`${operation.id}-${index}`} className="text-xs font-mono text-gray-400">
                  <span className="text-gray-600">[{log.timestamp.toLocaleTimeString()}]</span>{' '}
                  <span
                    className={
                      log.type === 'error'
                        ? 'text-red-400'
                        : log.type === 'success'
                          ? 'text-green-400'
                          : log.type === 'warning'
                            ? 'text-yellow-400'
                            : 'text-gray-400'
                    }
                  >
                    {log.message}
                  </span>
                </div>
              ))}
            </div>
          )}
        </motion.div>
      ))}
    </div>
  )
}

export function TaskHubModal({
  isOpen,
  onClose,
  operations,
  onCancelOperation,
}: {
  isOpen: boolean
  onClose: () => void
  operations: RunningOperation[]
  onCancelOperation: (id: string) => void
}) {
  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
          onClick={onClose}
        >
          <motion.div
            initial={{ scale: 0.95, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.95, opacity: 0 }}
            className="bg-[#0A0A0A] rounded-xl border border-[#27272A] max-w-2xl w-full max-h-[80vh] overflow-hidden"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="p-4 border-b border-[#27272A] flex items-center justify-between">
              <h2 className="text-lg font-semibold text-white flex items-center gap-2">
                <TerminalIcon size={20} />
                Task Hub
              </h2>
              <button onClick={onClose} className="text-gray-400 hover:text-white">
                <X size={20} />
              </button>
            </div>
            <TaskHub operations={operations} onCancelOperation={onCancelOperation} />
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
