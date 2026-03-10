import { RotateCcw } from 'lucide-react'
import { PackageCard } from '../components/PackageCard'
import { EmptyState, SkeletonCard, SkeletonList } from '../components/shared'
import type { Package } from '../types'

export function UpdatesPage({
  loading,
  viewMode,
  filteredUpdates,
  updateProgress,
  onAction,
  onUpdateAll,
}: {
  loading: boolean
  viewMode: 'grid' | 'list'
  filteredUpdates: Package[]
  updateProgress: number
  onAction: (action: 'install' | 'remove' | 'update', pkg: Package) => void
  onUpdateAll: () => void
}) {
  if (loading) {
    return viewMode === 'grid' ? (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {[...Array(4)].map((_, index) => (
          <SkeletonCard key={index} />
        ))}
      </div>
    ) : (
      <SkeletonList />
    )
  }

  if (filteredUpdates.length === 0) {
    return (
      <EmptyState
        icon={RotateCcw}
        title="All packages are up to date"
        description="No updates available at the moment."
      />
    )
  }

  return (
    <>
      <div className="mb-6 p-4 bg-gradient-to-r from-yellow-900/20 to-orange-900/20 rounded-xl border border-yellow-900/30">
        <div className="flex items-center justify-between mb-3">
          <div>
            <h3 className="text-lg font-semibold text-white">{filteredUpdates.length} updates available</h3>
            <p className="text-sm text-gray-400">Keep your system secure and up-to-date</p>
          </div>
          <button
            onClick={onUpdateAll}
            className="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg transition-colors flex items-center gap-2"
          >
            <RotateCcw size={16} />
            Update All
          </button>
        </div>
        {updateProgress > 0 && (
          <div className="mt-3">
            <div className="flex justify-between text-sm text-gray-400 mb-1">
              <span>Progress</span>
              <span>{updateProgress}/{filteredUpdates.length}</span>
            </div>
            <div className="h-2 bg-[#1A1A1A] rounded-full overflow-hidden">
              <div
                className="h-full bg-yellow-500 transition-all duration-300"
                style={{ width: `${(updateProgress / filteredUpdates.length) * 100}%` }}
              />
            </div>
          </div>
        )}
      </div>

      <div className={viewMode === 'grid' ? 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4' : 'space-y-3'}>
        {filteredUpdates.map((pkg) => (
          <PackageCard key={`${pkg.name}-${pkg.source}`} pkg={pkg} viewMode={viewMode} onAction={onAction} />
        ))}
      </div>
    </>
  )
}
