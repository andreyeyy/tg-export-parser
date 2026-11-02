## TgExportParser
Simple library for parsing a Telegram export file (result.json)

Why this library? It lets you parse result.json message by message, without storing
the entire file in memory at once. This makes it possible to efficiently parse
files of any size, while other parsers would run out of memory.

#### Example

```rust
use std::path::Path;

use tg_export_parser::TgExportParser;

fn main() {

    let path = Path::new("/path/to/result.json");

    let messages = TgExportParser::from_file(path).unwrap();

    for message in messages {
        // Process messages one by one here
        
        // For example, print all messages
        println!("{:?}", message.unwrap().text);
    }
}
```

