import { useEffect } from 'react'

export function useKeyboardShortcuts({
  onShowShortcuts,
  onToggleTaskHub,
  onCloseOverlays,
}: {
  onShowShortcuts: () => void
  onToggleTaskHub: () => void
  onCloseOverlays: () => void
}) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === '?' && !event.ctrlKey && !event.metaKey) {
        event.preventDefault()
        onShowShortcuts()
      }
      if (event.key === 't' && !event.ctrlKey && !event.metaKey) {
        event.preventDefault()
        onToggleTaskHub()
      }
      if (event.key === 'Escape') {
        onCloseOverlays()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [onCloseOverlays, onShowShortcuts, onToggleTaskHub])
}
