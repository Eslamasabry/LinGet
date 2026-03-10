import type { ElementType, ReactNode } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
import { AlertCircle, AlertTriangle, Check, Terminal, X } from 'lucide-react'
import type { Toast } from '../types'

export function PageTransition({ children }: { children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -10 }}
      transition={{ duration: 0.2 }}
    >
      {children}
    </motion.div>
  )
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
}: {
  icon: ElementType
  title: string
  description: string
  action?: ReactNode
}) {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      className="flex flex-col items-center justify-center h-64"
    >
      <div className="p-4 bg-[#0A0A0A] rounded-full mb-4">
        <Icon size={48} className="text-gray-600" />
      </div>
      <h3 className="text-lg font-medium text-white mb-1">{title}</h3>
      <p className="text-sm text-gray-500 text-center max-w-sm">{description}</p>
      {action && <div className="mt-4">{action}</div>}
    </motion.div>
  )
}

export function SkeletonCard() {
  return (
    <div className="bg-[#0A0A0A] rounded-lg border border-[#27272A] p-4 animate-pulse">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="h-4 bg-[#1A1A1A] rounded w-1/2 mb-2"></div>
          <div className="h-3 bg-[#1A1A1A] rounded w-1/3 mb-3"></div>
          <div className="h-3 bg-[#1A1A1A] rounded w-3/4"></div>
        </div>
      </div>
    </div>
  )
}

export function SkeletonList() {
  return (
    <div className="space-y-3">
      {[...Array(5)].map((_, index) => (
        <div key={index} className="flex items-center gap-3 p-3 bg-[#0A0A0A] rounded-lg animate-pulse">
          <div className="w-10 h-10 bg-[#1A1A1A] rounded-lg"></div>
          <div className="flex-1">
            <div className="h-4 bg-[#1A1A1A] rounded w-1/3 mb-2"></div>
            <div className="h-3 bg-[#1A1A1A] rounded w-1/4"></div>
          </div>
        </div>
      ))}
    </div>
  )
}

export function ToastContainer({
  toasts,
  onDismiss,
}: {
  toasts: Toast[]
  onDismiss: (id: number) => void
}) {
  return (
    <div className="fixed bottom-4 right-4 z-50 space-y-2">
      <AnimatePresence>
        {toasts.map((toast) => (
          <motion.div
            key={toast.id}
            initial={{ opacity: 0, x: 50 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 50 }}
            className={`px-4 py-3 rounded-lg border flex items-center gap-3 min-w-[320px] shadow-lg ${
              toast.type === 'success'
                ? 'bg-[#022c22] border-green-800 text-green-400'
                : toast.type === 'error'
                  ? 'bg-[#450a0a] border-red-800 text-red-400'
                  : toast.type === 'warning'
                    ? 'bg-[#451a03] border-yellow-800 text-yellow-400'
                    : 'bg-[#0c4a6e] border-blue-800 text-blue-400'
            }`}
          >
            {toast.type === 'success' ? <Check size={18} /> : toast.type === 'error' ? <AlertCircle size={18} /> : toast.type === 'warning' ? <AlertTriangle size={18} /> : <Terminal size={18} />}
            <span className="flex-1 text-sm">{toast.message}</span>
            <button onClick={() => onDismiss(toast.id)} className="text-gray-400 hover:text-white transition-colors">
              <X size={14} />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  )
}
