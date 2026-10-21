/*
+----------------------+
| Magic Number (4b)    | "1CV8"
+----------------------+
| Version (4b)         |
+----------------------+
| String Table         | // All identifiers, literals, method names
+----------------------+
| Metadata Ref Table   | // References to pre-compiled metadata (Directories, Documents)
+----------------------+
| Function Table       | // Offsets to functions/methods in code section, entry point, module variables
+----------------------+
| Code Section         | // Linear bytecode array
+----------------------+
| Debug Info (Opt)     | // Line numbers; preceded by a presence byte
+----------------------+

Every section starts with its own 4 byte magic and a u32 version. All integers are little endian.
*/
mod code_section;
mod debug_info;
mod function_table;
mod metadata_ref_table;
mod object_file;
mod string_table;
mod table_trait;

pub use code_section::CodeSection;
pub use debug_info::DebugInfo;
pub use function_table::{FunctionEntry, FunctionTable};
pub use metadata_ref_table::MetadataRefTable;
pub use object_file::{ObjectFile, MAGIC_NUMBER, OBJECT_FILE_VERSION};
pub use string_table::StringTable;
pub use table_trait::{Deserializable, Serializable};
