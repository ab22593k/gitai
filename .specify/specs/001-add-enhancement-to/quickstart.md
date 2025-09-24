# Quickstart Test: Enhancement to git-wire to avoid multiple git pulls for the same repository

## Setup
1. Install git-wire with the new enhancement
2. Create a configuration file (e.g., `.git-wire.json`) with multiple entries for the same repository:
   ```json
   {
     "repositories": [
       {
         "url": "https://github.com/example/repo.git",
         "branch": "main",
         "target_path": "./src/module1",
         "filters": ["src/", "lib/"]
       },
       {
         "url": "https://github.com/example/repo.git",  // Same repo
         "branch": "main",
         "target_path": "./src/module2",
         "filters": ["utils/"]
       }
     ]
   }
   ```

## Test Execution
1. Run the sync command: `git wire sync`
2. Observe that the repository is pulled only once
3. Verify that both target paths contain the expected content from the repository
4. Check the performance improvement compared to the old approach

## Expected Results
- Git-wire should detect the duplicate repository entries
- Only one git pull operation should be performed for the example repository
- Both ./src/module1 and ./src/module2 should contain the appropriate files from the repository based on their filters
- The sync operation should complete faster than with the old approach
- The cache should be located in a temporary directory and properly cleaned up after the operation