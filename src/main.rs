use clap::Parser as ClapParser;
use combine::error::StringStreamError;
use combine::parser::char::{char, letter, space};
use combine::{Parser, any, many, many1, optional, skip_count, token};
use std::collections::LinkedList;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, ClapParser)]
#[command(
    version,
    name = "exodocia",
    about = "Simple documentation extraction tool.",
    long_about = "Simple tool for extracting documentation from _any_ source code. Specifically shell scripts"
)]
struct Opt {
    #[arg(value_name = "SOURCE_FILE")]
    input: PathBuf,

    #[arg(long, default_value = "##")]
    doc_comment_identifier: String,
}

type Source = LinkedList<SourceHunk>;

fn to_source_elements(source_text: String) -> Source {
    let doc_prefix = "##";
    let mut source_lines: Source = LinkedList::new();

    for source_line in source_text.lines() {
        if source_line.trim().starts_with(doc_prefix) {
            source_lines.push_back(SourceHunk::Doc(
                source_line.trim_start_matches(doc_prefix).trim().to_owned(),
            ));
        } else {
            source_lines.push_back(SourceHunk::Code(source_line.to_owned()));
        }
    }

    source_lines
}

fn meld_neighbors(source_lines: Source) -> Source {
    let mut part: String = "".to_owned();
    let mut documentation: Source = LinkedList::new();

    let mut last_part_type = match source_lines.front() {
        Some(SourceHunk::Code(_)) => HunkType::Code,
        Some(SourceHunk::Doc(_)) => HunkType::Doc,
        _ => return documentation,
    };

    for line in source_lines.iter() {
        match line {
            SourceHunk::Code(line_str) => {
                if HunkType::Code == last_part_type {
                    if !part.is_empty() {
                        part.push_str("\n");
                    }
                    part.push_str(line_str);
                } else {
                    documentation.push_back(SourceHunk::Doc(part));

                    part = line_str.to_owned();
                    last_part_type = HunkType::Code;
                }
            }
            SourceHunk::Doc(line_str) => {
                if HunkType::Doc == last_part_type {
                    if !part.is_empty() {
                        part.push_str("\n");
                    }
                    part.push_str(line_str);
                } else {
                    documentation.push_back(SourceHunk::Code(part));

                    part = line_str.to_owned();
                    last_part_type = HunkType::Doc;
                }
            }
        }
    }
    match last_part_type {
        HunkType::Code => documentation.push_back(SourceHunk::Code(part)),
        HunkType::Doc => documentation.push_back(SourceHunk::Doc(part)),
    }

    documentation
}

#[derive(Debug, PartialEq)]
enum SourceHunk {
    Code(String),
    Doc(String),
}

#[derive(Debug, PartialEq)]
enum HunkType {
    Code,
    Doc,
}

#[derive(Clone, Debug, PartialEq)]
struct DocEntry {
    name: String,
    content: String,
}

impl DocEntry {
    fn parse_from(text: &str) -> Result<LinkedList<DocEntry>, StringStreamError> {
        let implicite_brief = many::<String, _, _>(any::<&str>()).skip(char('@'));

        let entry = skip_count(1, token('@'))
            .with(many1::<String, _, _>(letter()))
            .skip(space())
            .skip(space())
            .and(many1::<String, _, _>(any()))
            .map(|(name, content)| DocEntry {
                name: name.to_owned(),
                content: content.to_owned(),
            });

        let mut entries = optional(implicite_brief).and(many(entry)).map(
            |(iml_brief_text, rest_entries): (Option<String>, LinkedList<DocEntry>)| {
                let mut res = rest_entries.clone();
                if let Some(brief_text) = iml_brief_text {
                    res.push_front(DocEntry {
                        name: "brief".to_owned(),
                        content: brief_text,
                    });
                }

                res
            },
        );

        entries.parse(text).map(|x| x.0)
    }
}

#[derive(Debug)]
enum Entry {
    Code(String),
    Doc(LinkedList<DocEntry>),
}

impl Entry {
    fn from(hunk: SourceHunk) -> Entry {
        match hunk {
            SourceHunk::Code(text) => Entry::Code(text),
            SourceHunk::Doc(text) => Entry::Doc(match DocEntry::parse_from(&text) {
                Ok(entries) => entries,
                Err(error_text) => panic!("Oops {:?}", error_text),
            }),
        }
    }
}

fn main() {
    let opt = Opt::parse();
    println!("Hello, world! Opts: {:?}", opt);
    println!("File name: {}", opt.input.display());

    let content = fs::read_to_string(opt.input).expect("Cannot load source file");
    println!("I've got a file: \"\"\"\n{content}\"\"\"");

    let parts_of_source = meld_neighbors(to_source_elements(content));
    for part in parts_of_source {
        println!("-> {:?}", Entry::from(part));
    }
}
