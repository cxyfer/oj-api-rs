# Project Context: oj-api-rs

## Language

| Term | Definition | Avoid |
|------|-----------|-------|
| Online Judge Source | A competitive-programming platform from which problem data and daily challenges originate. | provider, website, crawler target |
| Unified Problem Record | A normalized representation of one programming problem that can be queried consistently across Online Judge Sources. | scraped item, raw problem, platform payload |
| Smart Resolution | The capability that turns a supported URL, slug, prefixed identifier, or recognizable problem ID into a concrete Unified Problem Record. | URL parser, ID guessing, redirect |
| Semantic Similarity Search | Meaning-based retrieval that ranks related programming problems from either an existing problem or a natural-language query. | keyword search, duplicate finder, AI search |
| Integration Surface | A supported way for software or AI tools to access the same problem intelligence and service capabilities. | frontend, endpoint collection, connector |
| Problem Intelligence Infrastructure | The unified foundation that lets people and AI systems resolve, retrieve, compare, and explore algorithmic problems across Online Judge Sources. | problem database, API wrapper, crawler service |
| Spatial Intelligence Map | An interactive spatial view that presents Online Judge Sources, algorithmic problems, and semantic relationships as a navigable information field. | particle background, 3D globe, decorative constellation |

## Relationships

- An **Online Judge Source** contributes many **Unified Problem Records**.
- **Smart Resolution** identifies a **Unified Problem Record** across supported input formats.
- **Semantic Similarity Search** ranks **Unified Problem Records** by conceptual relatedness.
- An **Integration Surface** exposes the service's problem intelligence to developers and AI tools.
- **Problem Intelligence Infrastructure** connects Unified Problem Records, Smart Resolution, Semantic Similarity Search, and Integration Surfaces into one product capability.
- The **Spatial Intelligence Map** visualizes the relationships made available by the **Problem Intelligence Infrastructure**.

## Flagged Ambiguities

- "AI search" -> use **Semantic Similarity Search** when referring to meaning-based related-problem retrieval (2026-07-10).
- "API" -> use **Integration Surface** when the statement applies to both HTTP API and MCP access (2026-07-10).
- "problem database" -> use **Problem Intelligence Infrastructure** when describing the complete product rather than stored records alone (2026-07-10).
- "particle background" -> use **Spatial Intelligence Map** when referring to the meaningful interactive hero visualization (2026-07-10).
