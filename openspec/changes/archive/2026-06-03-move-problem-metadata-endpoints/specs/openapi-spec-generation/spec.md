## ADDED Requirements

### Requirement: Problem metadata discovery paths in generated OpenAPI
The generated OpenAPI document SHALL advertise problem metadata discovery under the `/api/v1/problems` namespace and SHALL NOT advertise the old top-level public metadata discovery paths.

#### Scenario: OpenAPI includes nested metadata discovery paths
- **WHEN** the OpenAPI document is generated
- **THEN** it includes `GET /api/v1/problems/tags/{source}`
- **AND** it includes `GET /api/v1/problems/difficulties/{source}`

#### Scenario: OpenAPI excludes old metadata discovery paths
- **WHEN** the OpenAPI document is generated
- **THEN** it does not include `GET /api/v1/tags/{source}`
- **AND** it does not include `GET /api/v1/difficulties/{source}`

#### Scenario: OpenAPI security remains bearer protected
- **WHEN** the OpenAPI document is generated
- **THEN** both nested metadata discovery operations declare the existing `bearer_auth` security requirement
