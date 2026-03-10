import { Package as PackageIcon } from 'lucide-react'
import { PackageCard } from '../components/PackageCard'
import { EmptyState, SkeletonCard, SkeletonList } from '../components/shared'
import type { Package } from '../types'

export function InstalledPage({
  loading,
  viewMode,
  filteredPackages,
  onAction,
}: {
  loading: boolean
  viewMode: 'grid' | 'list'
  filteredPackages: Package[]
  onAction: (action: 'install' | 'remove' | 'update', pkg: Package) => void
}) {
  if (loading) {
    return viewMode === 'grid' ? (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {[...Array(6)].map((_, index) => (
          <SkeletonCard key={index} />
        ))}
      </div>
    ) : (
      <SkeletonList />
    )
  }

  if (filteredPackages.length === 0) {
    return (
      <EmptyState
        icon={PackageIcon}
        title="No installed packages"
        description="No packages match your search or filter criteria."
      />
    )
  }

  return (
    <div className={viewMode === 'grid' ? 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4' : 'space-y-3'}>
      {filteredPackages.map((pkg) => (
        <PackageCard key={`${pkg.name}-${pkg.source}`} pkg={pkg} viewMode={viewMode} onAction={onAction} />
      ))}
    </div>
  )
}
