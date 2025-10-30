use std::path::Path;
use ignore::gitignore::Gitignore;

fn main() {
    let (gitignore, _) = Gitignore::new(Path::new(".gitignore"));
    let result = gitignore.matched(Path::new("node_modules/package.json"), false);
    println!("node_modules/package.json ignored: {}", result.is_ignore());
    
    let result2 = gitignore.matched(Path::new("node_modules/"), true);
    println!("node_modules/ ignored: {}", result2.is_ignore());
}
