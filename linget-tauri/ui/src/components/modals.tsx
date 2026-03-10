import { AnimatePresence, motion } from 'framer-motion'
import { AlertTriangle, ExternalLink, Info, Keyboard, Package as PackageIcon, X } from 'lucide-react'
import { SHORTCUTS } from '../constants'

export interface ConfirmDialogProps {
  isOpen: boolean
  title: string
  message: string
  confirmText: string
  cancelText: string
  confirmStyle: 'danger' | 'primary'
  onConfirm: () => void
  onCancel: () => void
}

export function ConfirmDialog({
  isOpen,
  title,
  message,
  confirmText,
  cancelText,
  confirmStyle,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
          onClick={onCancel}
        >
          <motion.div
            initial={{ scale: 0.95, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.95, opacity: 0 }}
            className="bg-[#0A0A0A] rounded-xl border border-[#27272A] max-w-md w-full p-6"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="flex items-start gap-4">
              <div className={`p-3 rounded-full ${confirmStyle === 'danger' ? 'bg-red-900/30' : 'bg-blue-900/30'}`}>
                {confirmStyle === 'danger' ? <AlertTriangle size={24} className="text-red-400" /> : <Info size={24} className="text-blue-400" />}
              </div>
              <div className="flex-1">
                <h3 className="text-lg font-semibold text-white mb-2">{title}</h3>
                <p className="text-gray-400 text-sm">{message}</p>
              </div>
            </div>
            <div className="flex gap-3 mt-6">
              <button
                onClick={onCancel}
                className="flex-1 px-4 py-2.5 bg-[#1A1A1A] text-gray-300 rounded-lg hover:bg-[#27272A] transition-colors"
              >
                {cancelText}
              </button>
              <button
                onClick={onConfirm}
                className={`flex-1 px-4 py-2.5 rounded-lg transition-colors font-medium ${
                  confirmStyle === 'danger' ? 'bg-red-600 text-white hover:bg-red-700' : 'bg-blue-600 text-white hover:bg-blue-700'
                }`}
              >
                {confirmText}
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

export function ShortcutsModal({ onClose }: { onClose: () => void }) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.95 }}
        animate={{ scale: 1 }}
        exit={{ scale: 0.95 }}
        className="bg-[#0A0A0A] rounded-xl border border-[#27272A] max-w-sm w-full p-6"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-white flex items-center gap-2">
            <Keyboard size={20} />
            Keyboard Shortcuts
          </h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white">
            <X size={20} />
          </button>
        </div>
        <div className="space-y-2">
          {SHORTCUTS.map((shortcut) => (
            <div key={shortcut.key} className="flex items-center justify-between py-2">
              <span className="text-gray-300">{shortcut.action}</span>
              <kbd className="px-2 py-1 text-xs bg-[#1A1A1A] text-gray-400 rounded border border-[#27272A]">
                {shortcut.key}
              </kbd>
            </div>
          ))}
        </div>
      </motion.div>
    </motion.div>
  )
}

export function AboutModal({ onClose }: { onClose: () => void }) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.95 }}
        animate={{ scale: 1 }}
        exit={{ scale: 0.95 }}
        className="bg-[#0A0A0A] rounded-xl border border-[#27272A] max-w-md w-full p-6"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="text-center">
          <div className="w-16 h-16 bg-[#1A1A1A] rounded-2xl flex items-center justify-center mx-auto mb-4">
            <PackageIcon size={32} className="text-blue-500" />
          </div>
          <h2 className="text-xl font-bold text-white mb-1">LinGet</h2>
          <p className="text-sm text-gray-500 mb-4">A modern GUI package manager for Linux</p>
          <div className="text-xs text-gray-600 space-y-1 mb-6">
            <p>Version 0.1.0</p>
            <p>Built with Tauri 2 + React</p>
          </div>
          <div className="flex justify-center gap-4">
            <a
              href="https://github.com/Eslamasabry/LinGet"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-400 hover:underline text-sm flex items-center gap-1"
            >
              <ExternalLink size={14} />
              GitHub
            </a>
          </div>
        </div>
        <button
          onClick={onClose}
          className="w-full mt-6 px-4 py-2 bg-[#1A1A1A] text-gray-300 rounded-lg hover:bg-[#27272A] transition-colors"
        >
          Close
        </button>
      </motion.div>
    </motion.div>
  )
}
