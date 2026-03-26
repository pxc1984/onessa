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
| Function Table       | // Offsets to functions/methods in code section
+----------------------+
| Code Section         | // Linear bytecode array
+----------------------+
| Debug Info (Opt)     | // Line numbers, variable names
+----------------------+
*/
mod code_section;
mod function_table;
mod metadata_ref_table;
mod object_file;
mod string_table;
mod table_trait;

pub use object_file::ObjectFile;
