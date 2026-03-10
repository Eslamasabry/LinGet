import { useCallback, useState } from 'react'
import type { Toast } from '../types'

export function useToasts() {
  const [toasts, setToasts] = useState<Toast[]>([])

  const dismissToast = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id))
  }, [])

  const showToast = useCallback((type: Toast['type'], message: string) => {
    const id = Date.now()
    setToasts((current) => [...current, { id, type, message }])
    window.setTimeout(() => {
      setToasts((current) => current.filter((toast) => toast.id !== id))
    }, 4000)
  }, [])

  return { toasts, showToast, dismissToast }
}
