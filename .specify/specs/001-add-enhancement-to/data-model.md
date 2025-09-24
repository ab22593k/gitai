# Data Model: Enhancement to git-wire to avoid multiple git pulls for the same repository

## Entities

### RepositoryConfiguration
- **Definition**: A definition of a remote git repository to be wired into the local project, including URL, branch, and target path
- **Attributes**:
  - `url`: String - The URL of the remote repository
  - `branch`: String - The branch to pull from (default: main/master)
  - `target_path`: String - The local path where content should be placed
  - `filters`: List<String> - Paths/filenames to include from the repository
  - `commit_hash`: Option<String> - Specific commit to check out (optional)

### CachedRepository
- **Definition**: A local copy of a remote repository that is used to source content for multiple wire operations
- **Attributes**:
  - `url`: String - The URL of the source repository
  - `branch`: String - The branch that was pulled
  - `local_cache_path`: String - The path where the cached repository is stored
  - `last_pulled`: DateTime - Timestamp of the last pull operation
  - `commit_hash`: String - The commit hash of the cached repository
  - `lock`: Mutex - To prevent concurrent access during operations

### WireOperation
- **Definition**: The process of extracting specific content from a remote repository and placing it in the local project
- **Attributes**:
  - `source_config`: RepositoryConfiguration - The configuration defining the source
  - `cached_repo_path`: String - Path to the cached repository to use
  - `operation_id`: UUID - Unique identifier for this operation

## Relationships
- One `CachedRepository` can be used by multiple `WireOperation` instances
- Multiple `RepositoryConfiguration` entries may map to the same `CachedRepository` if they reference the same remote repository

## State Transitions
### CachedRepository
- `Not Cached` → `Pulling` → `Cached` → `In Use` → `Available` → `Expired` → `Refreshed`

## Validation Rules
- Repository URL must be a valid git URL
- Target path must be within project boundaries
- Filters must reference valid paths within the source repository
- Cache expiration time must be within reasonable bounds