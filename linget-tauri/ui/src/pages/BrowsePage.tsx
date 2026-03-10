import { Search } from 'lucide-react'
import { PackageCard } from '../components/PackageCard'
import { EmptyState, SkeletonCard, SkeletonList } from '../components/shared'
import type { Package } from '../types'

export function BrowsePage({
  loading,
  viewMode,
  searchQuery,
  filteredBrowsePackages,
  onAction,
}: {
  loading: boolean
  viewMode: 'grid' | 'list'
  searchQuery: string
  filteredBrowsePackages: Package[]
  onAction: (action: 'install' | 'remove' | 'update', pkg: Package) => void
}) {
  if (loading && searchQuery) {
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

  if (!searchQuery) {
    return (
      <EmptyState
        icon={Search}
        title="Search for packages"
        description="Enter a package name or keyword to discover software across your enabled sources."
      />
    )
  }

  if (filteredBrowsePackages.length === 0) {
    return (
      <EmptyState
        icon={Search}
        title="No packages found"
        description={`No packages match "${searchQuery}". Try another search term.`}
      />
    )
  }

  return (
    <div className={viewMode === 'grid' ? 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4' : 'space-y-3'}>
      {filteredBrowsePackages.map((pkg) => (
        <PackageCard key={`${pkg.name}-${pkg.source}`} pkg={pkg} viewMode={viewMode} onAction={onAction} />
      ))}
    </div>
  )
}
