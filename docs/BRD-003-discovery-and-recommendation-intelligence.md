## BRD-003 — Discovery and Recommendation Intelligence

### 0) TUI-First Delivery Scope
- Current implementation target: **TUI first**.
- Discovery, ranking, explanation, and collection behaviors are implemented in TUI list/detail surfaces first.
- GUI/CLI are deferred to later phases and should remain behaviorally compatible.

### 1) Strategic Intent
- Improve package discovery from noisy ecosystem lists to intent-driven recommendations.
- Help users find the right package quickly while preserving explicit control.

### 2) Current Problem
- Users face search friction across many providers and naming schemes.
- Existing discovery is mostly string matching, not intent-aware.

### 3) Scope
- Search/ranking engine and collection system across supported providers.
- Recommendation surfaces, companion package suggestions, and source trust/freshness signals.
- Offline/low-connectivity discoverability paths.

### 4) Success Definition
- Default discovery surfaces relevant packages faster with fewer false positives.
- Users can explore curated categories and understand why a result is recommended.

### 5) Requirements

#### 5.1 Functional Requirements
- FR1: Add ranked scoring for search results using exact match, tokens, metadata relevance, freshness, trust, and provider preference.
- FR2: Add typo tolerance, stemming/alias normalization, and synonym expansion for common names.
- FR3: Add discoverable collections (`developer-tools`, `gaming`, `media`, `education`, `sysadmin`, `containers`, `security-first`).
- FR4: Expose companion recommendations for major package installs where known.
- FR5: Surface changelog/security relevance signals in search results for updated packages.
- FR6: Add stale-source/last-fetched freshness badges to search results.
- FR7: Add cached/disconnected index mode for searchable local metadata.

#### 5.2 Non-Functional Requirements
- NFR1: Search ranking should remain responsive at scale (`< 200ms` on warm cache for common queries).
- NFR2: Recommendation changes should be deterministic for same input unless index version changes.
- NFR3: Rank explainability: user can inspect top ranking contributors.

### 6) Frontend Requirements
- GUI
  - Introduce multi-pane discovery with ranked cards + rank explanation panel.
  - Add collection shortcuts and trust/freshness chips on each card.
  - Show “why this matches” tooltip or drawer.
- TUI
  - Keep discovery dense but scannable: score column, source badge, freshness marker, recommendation tag.
  - Add `/` smart search that supports collection jump and quick filters.
  - Add command for recommendation reason/details per item.
- CLI
  - Add `linget search --recommend --collection --explain` options.
  - Include score components in JSON responses for automation and debugging.

### 7) Data Model (Initial)
- `DiscoveryIndex`
  - `provider`, `package_id`, `normalized_tokens`, `categories`, `aliases`, `use_cases`, `trust_score`, `freshness`, `last_refresh_at`, `score_weights_version`
- `RecommendationContext`
  - `context_name`, `input_packages`, `user_profile`, `allowed_sources`, `risk_tolerance`, `generated_at`, `expires_at`

### 8) Risks
- Ranking bias toward popularity may hurt niche package discoverability.
- Bad alias data can produce false recommendations.
- Offline cache staleness could mislead users if freshness is not clearly surfaced.

### 9) Acceptance Criteria
- P0: A seeded set of 20 intent queries returns expected top candidates with > 80% relevance rating in manual review.
- P1: Search latency and ranking remain within target under synthetic large dataset tests.
- P2: Freshness and trust indicators are visible and consistent across frontends.

### 10) Additional Cross-Cutting Coverage
- RQ-20: Offline/low-connectivity discovery cache should clearly display freshness and staleness.
- RQ-22: Discoverability analytics are opt-in and redacted by default (privacy-safe by design).
- RQ-23: Recommendation text and collection names prepared for localization.
- RQ-24: Onboarding copy explains search/recommendation modes during first run or first discovery action.

### 11) KPIs
- Discovery completion time (query-to-install decision) improved.
- Higher conversion from search/recommendation path to installation.
- Reduced “package not found” and repeated query loops.
