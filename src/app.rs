//! CLI wiring: argument parsing, dispatch, envelope emit, autosave, and
//! session glue. No business logic lives here (`AGENTS.md`).
//!
//! Parity notes: `--version`/`--help` and usage errors behave like Typer —
//! plain text, non-JSON, clap's exit codes (0/2). No subcommand prints the
//! howto text. `document new/open/save/close` manage the session exactly like
//! Words' typer wrappers; all other successful categories auto-save the open
//! document best-effort (Words' `_AUTOSAVE`).

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::commands::{self, Category};
use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::core::output::render;
use crate::core::session::Session;
use crate::howto;
use crate::models::data::Data;

/// Poet - AI-First Word Document (.docx) Automation CLI
#[derive(Debug, Parser)]
#[command(name = "poet", version, disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print a comprehensive system prompt describing all commands.
    Howto,
    /// Document management commands.
    Document(CategoryArgs<commands::document::DocumentAction>),
    /// Section commands.
    Section(CategoryArgs<commands::section::SectionAction>),
    /// Paragraph operations commands.
    Paragraph(CategoryArgs<commands::paragraph::ParagraphAction>),
    /// Run (text span) commands.
    Run(CategoryArgs<commands::run::RunAction>),
    /// Style commands.
    Style(CategoryArgs<commands::style::StyleAction>),
    /// Heading commands.
    Heading(CategoryArgs<commands::heading::HeadingAction>),
    /// List commands.
    List(CategoryArgs<commands::list::ListAction>),
    /// Table commands.
    Table(CategoryArgs<commands::table::TableAction>),
    /// Image commands.
    Image(CategoryArgs<commands::image::ImageAction>),
    /// Table of contents commands.
    Toc(CategoryArgs<commands::toc::TocAction>),
    /// Page layout commands.
    Page(CategoryArgs<commands::page::PageAction>),
    /// Metadata commands.
    Meta(CategoryArgs<commands::meta::MetaAction>),
    /// Batch processing commands.
    Batch(CategoryArgs<commands::batch::BatchAction>),
    /// Calculation and analysis commands.
    Calc(CategoryArgs<commands::calc::CalcAction>),
}

/// Wrapper turning a category's action enum into clap `Args`.
#[derive(Debug, clap::Args)]
struct CategoryArgs<A: clap::Subcommand> {
    #[command(subcommand)]
    action: A,
}

/// Process entry: build the production context, execute, print, exit.
pub fn run<I>(args: I) -> ExitCode
where
    I: IntoIterator<Item = String>,
{
    let mut argv = vec!["poet".to_string()];
    argv.extend(args);
    let ctx = Ctx::from_env();
    let (json, code) = run_with(ctx, argv);
    if let Some(line) = json {
        println!("{line}");
    }
    code
}

/// In-process execution for tests: parse + dispatch + emit. Returns the
/// output text (`None` when clap rendered help/version itself) and the
/// process exit code.
pub fn run_with<I>(ctx: Ctx, argv: I) -> (Option<String>, ExitCode)
where
    I: IntoIterator<Item = String>,
{
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            let code = u8::try_from(e.exit_code()).unwrap_or(1);
            return (None, ExitCode::from(code));
        }
    };
    let Some(command) = cli.command else {
        return (Some(howto::HOWTO.to_string()), ExitCode::SUCCESS);
    };
    if matches!(command, Commands::Howto) {
        // Words prints the howto as raw text, not a JSON envelope.
        return (Some(howto::HOWTO.to_string()), ExitCode::SUCCESS);
    }
    let (category, result) = dispatch(&ctx, command);
    let (json, is_error) = render(&result);
    if !is_error && category.autosaves() {
        autosave(&ctx);
    }
    let code = if is_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    };
    (Some(json), code)
}

/// Persist the open document to its current path, ignoring failures
/// (Words' autosave is silent best-effort).
fn autosave(ctx: &Ctx) {
    let mut doc = ctx.doc.borrow_mut();
    if let Some(mgr) = doc.as_mut()
        && let Some(path) = mgr.current_path().map(|p| p.to_path_buf())
    {
        let _ = mgr.save("docx", path);
    }
}
fn start_session(ctx: &Ctx, path: &str) -> Result<(), PoetError> {
    ctx.session.save(&Session {
        path: path.into(),
        format: "docx".into(),
        active_section: None,
    })
}

fn end_session(ctx: &Ctx) -> Result<(), PoetError> {
    ctx.session.delete()
}

fn dispatch(ctx: &Ctx, command: Commands) -> (Category, Result<Data, PoetError>) {
    match command {
        // `Howto` is intercepted in `run_with` (raw-text output, not an
        // envelope); this arm only keeps the match exhaustive.
        Commands::Howto => (
            Category::Document,
            Err(PoetError::Internal("howto handled at parse layer".into())),
        ),
        Commands::Document(args) => match args.action {
            commands::document::DocumentAction::New(args) => {
                // Words' typer wrapper: create, persist so `&&` continues,
                // then start the session; any failure surfaces as the error.
                let result = commands::document::new(ctx, &args).and_then(|data| {
                    {
                        let mut doc = ctx.doc.borrow_mut();
                        let mgr = doc.as_mut().ok_or_else(|| {
                            PoetError::Internal("document missing after create".into())
                        })?;
                        mgr.save("docx", &args.path)?;
                    }
                    start_session(ctx, &args.path)?;
                    Ok(data)
                });
                (Category::Document, result)
            }
            commands::document::DocumentAction::Open(args) => {
                let result = commands::document::open(ctx, &args)
                    .and_then(|data| start_session(ctx, &args.path).map(|_| data));
                (Category::Document, result)
            }
            commands::document::DocumentAction::Save(args) => {
                let result = commands::document::save(ctx, &args).and_then(|data| {
                    let saved = args.path.clone().or_else(|| {
                        ctx.doc
                            .borrow()
                            .as_ref()
                            .and_then(|mgr| mgr.current_path())
                            .map(|p| p.to_string_lossy().into_owned())
                    });
                    if let Some(saved) = saved {
                        start_session(ctx, &saved)?;
                    }
                    Ok(data)
                });
                (Category::Document, result)
            }
            commands::document::DocumentAction::Close(args) => {
                let result = commands::document::close(ctx, &args)
                    .and_then(|data| end_session(ctx).map(|_| data));
                (Category::Document, result)
            }
            commands::document::DocumentAction::Info(args) => {
                (Category::Document, commands::document::info(ctx, &args))
            }
            commands::document::DocumentAction::Export(args) => {
                (Category::Document, commands::document::export(ctx, &args))
            }
        },
        Commands::Section(args) => match args.action {
            commands::section::SectionAction::List(a) => {
                (Category::Section, commands::section::list(ctx, &a))
            }
            commands::section::SectionAction::Info(a) => {
                (Category::Section, commands::section::info(ctx, &a))
            }
            commands::section::SectionAction::Add(a) => {
                (Category::Section, commands::section::add(ctx, &a))
            }
            commands::section::SectionAction::PageBreak(a) => {
                (Category::Section, commands::section::page_break(ctx, &a))
            }
        },
        Commands::Paragraph(args) => match args.action {
            commands::paragraph::ParagraphAction::Add(a) => {
                (Category::Paragraph, commands::paragraph::add(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Insert(a) => {
                (Category::Paragraph, commands::paragraph::insert(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Get(a) => {
                (Category::Paragraph, commands::paragraph::get(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Update(a) => {
                (Category::Paragraph, commands::paragraph::update(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Delete(a) => {
                (Category::Paragraph, commands::paragraph::delete(ctx, &a))
            }
            commands::paragraph::ParagraphAction::List(a) => {
                (Category::Paragraph, commands::paragraph::list(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Move(a) => {
                (Category::Paragraph, commands::paragraph::r#move(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Clear(a) => {
                (Category::Paragraph, commands::paragraph::clear(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Border(a) => {
                (Category::Paragraph, commands::paragraph::border(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Find(a) => {
                (Category::Paragraph, commands::paragraph::find(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Replace(a) => {
                (Category::Paragraph, commands::paragraph::replace(ctx, &a))
            }
            commands::paragraph::ParagraphAction::Count(a) => {
                (Category::Paragraph, commands::paragraph::count(ctx, &a))
            }
        },
        Commands::Run(args) => match args.action {
            commands::run::RunAction::Add(a) => (Category::Run, commands::run::add(ctx, &a)),
            commands::run::RunAction::Get(a) => (Category::Run, commands::run::get(ctx, &a)),
            commands::run::RunAction::Clear(a) => (Category::Run, commands::run::clear(ctx, &a)),
            commands::run::RunAction::Format(a) => (Category::Run, commands::run::format(ctx, &a)),
            commands::run::RunAction::Emphasize(a) => {
                (Category::Run, commands::run::emphasize(ctx, &a))
            }
        },
        Commands::Style(args) => match args.action {
            commands::style::StyleAction::List(a) => {
                (Category::Style, commands::style::list(ctx, &a))
            }
            commands::style::StyleAction::Apply(a) => {
                (Category::Style, commands::style::apply(ctx, &a))
            }
        },
        Commands::Heading(args) => match args.action {
            commands::heading::HeadingAction::Add(a) => {
                (Category::Heading, commands::heading::add(ctx, &a))
            }
            commands::heading::HeadingAction::SetLevel(a) => {
                (Category::Heading, commands::heading::set_level(ctx, &a))
            }
            commands::heading::HeadingAction::List(a) => {
                (Category::Heading, commands::heading::list(ctx, &a))
            }
        },
        Commands::List(args) => match args.action {
            commands::list::ListAction::Add(a) => (Category::List, commands::list::add(ctx, &a)),
            commands::list::ListAction::AddItem(a) => {
                (Category::List, commands::list::add_item(ctx, &a))
            }
            commands::list::ListAction::Convert(a) => {
                (Category::List, commands::list::convert(ctx, &a))
            }
            commands::list::ListAction::SetLevel(a) => {
                (Category::List, commands::list::set_level(ctx, &a))
            }
        },
        Commands::Table(args) => match args.action {
            commands::table::TableAction::Add(a) => {
                (Category::Table, commands::table::add(ctx, &a))
            }
            commands::table::TableAction::List(a) => {
                (Category::Table, commands::table::list(ctx, &a))
            }
            commands::table::TableAction::Get(a) => {
                (Category::Table, commands::table::get(ctx, &a))
            }
            commands::table::TableAction::SetCell(a) => {
                (Category::Table, commands::table::set_cell(ctx, &a))
            }
            commands::table::TableAction::SetRange(a) => {
                (Category::Table, commands::table::set_range(ctx, &a))
            }
            commands::table::TableAction::AddRow(a) => {
                (Category::Table, commands::table::add_row(ctx, &a))
            }
            commands::table::TableAction::AddColumn(a) => {
                (Category::Table, commands::table::add_column(ctx, &a))
            }
            commands::table::TableAction::DeleteRow(a) => {
                (Category::Table, commands::table::delete_row(ctx, &a))
            }
            commands::table::TableAction::DeleteColumn(a) => {
                (Category::Table, commands::table::delete_column(ctx, &a))
            }
        },
        Commands::Image(args) => match args.action {
            commands::image::ImageAction::Add(a) => {
                (Category::Image, commands::image::add(ctx, &a))
            }
            commands::image::ImageAction::List(a) => {
                (Category::Image, commands::image::list(ctx, &a))
            }
            commands::image::ImageAction::Get(a) => {
                (Category::Image, commands::image::get(ctx, &a))
            }
            commands::image::ImageAction::Resize(a) => {
                (Category::Image, commands::image::resize(ctx, &a))
            }
            commands::image::ImageAction::Delete(a) => {
                (Category::Image, commands::image::delete(ctx, &a))
            }
        },
        Commands::Toc(args) => match args.action {
            commands::toc::TocAction::Add(a) => (Category::Toc, commands::toc::add(ctx, &a)),
            commands::toc::TocAction::Update(a) => (Category::Toc, commands::toc::update(ctx, &a)),
        },
        Commands::Page(args) => match args.action {
            commands::page::PageAction::Margins(a) => {
                (Category::Page, commands::page::margins(ctx, &a))
            }
            commands::page::PageAction::Orientation(a) => {
                (Category::Page, commands::page::orientation(ctx, &a))
            }
            commands::page::PageAction::Size(a) => (Category::Page, commands::page::size(ctx, &a)),
            commands::page::PageAction::Header(a) => {
                (Category::Page, commands::page::header(ctx, &a))
            }
            commands::page::PageAction::Footer(a) => {
                (Category::Page, commands::page::footer(ctx, &a))
            }
            commands::page::PageAction::PageNumbers(a) => {
                (Category::Page, commands::page::page_numbers(ctx, &a))
            }
            commands::page::PageAction::Columns(a) => {
                (Category::Page, commands::page::columns(ctx, &a))
            }
        },
        Commands::Meta(args) => match args.action {
            commands::meta::MetaAction::Describe(a) => {
                (Category::Meta, commands::meta::describe(ctx, &a))
            }
            commands::meta::MetaAction::GetDocument(a) => {
                (Category::Meta, commands::meta::get_document(ctx, &a))
            }
            commands::meta::MetaAction::SetDocument(a) => {
                (Category::Meta, commands::meta::set_document(ctx, &a))
            }
            commands::meta::MetaAction::GetSection(a) => {
                (Category::Meta, commands::meta::get_section(ctx, &a))
            }
            commands::meta::MetaAction::SetSection(a) => {
                (Category::Meta, commands::meta::set_section(ctx, &a))
            }
            commands::meta::MetaAction::GetTable(a) => {
                (Category::Meta, commands::meta::get_table(ctx, &a))
            }
            commands::meta::MetaAction::SetTable(a) => {
                (Category::Meta, commands::meta::set_table(ctx, &a))
            }
            commands::meta::MetaAction::History(a) => {
                (Category::Meta, commands::meta::history(ctx, &a))
            }
        },
        Commands::Batch(args) => match args.action {
            commands::batch::BatchAction::Run(a) => {
                (Category::Batch, commands::batch::run(ctx, &a))
            }
            commands::batch::BatchAction::Template(a) => {
                (Category::Batch, commands::batch::template(ctx, &a))
            }
        },
        Commands::Calc(args) => match args.action {
            commands::calc::CalcAction::Read(a) => (Category::Calc, commands::calc::read(ctx, &a)),
            commands::calc::CalcAction::Stats(a) => {
                (Category::Calc, commands::calc::stats(ctx, &a))
            }
            commands::calc::CalcAction::Aggregate(a) => {
                (Category::Calc, commands::calc::aggregate(ctx, &a))
            }
            commands::calc::CalcAction::Filter(a) => {
                (Category::Calc, commands::calc::filter(ctx, &a))
            }
            commands::calc::CalcAction::Transform(a) => {
                (Category::Calc, commands::calc::transform(ctx, &a))
            }
        },
    }
}
