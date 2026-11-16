//! Table column-width probe: docx in → import result + layout decisions out (JSON).
//!
//! Purpose: numeric diagnosis of table layout bugs (#6574 pct ignored, #6769 tcW/grid
//! disagreement, autofit snapshots). Dumps, per table: the imported intent (props),
//! the tblGrid snapshot, per-cell/per-row props, and the layout engine's own column
//! measurements (`measure_table_columns`) plus `autofit_widths` at the text-column width.
//!
//! Usage:
//!   table_dump <file.docx>              # read docx, print probe JSON, write <file>.doc.json
//!   table_dump --json <file.doc.json>   # probe a previously dumped Document JSON (skip import)
//!
//! Twips→points: 20 twips = 1 pt; a 1-twip disagreement = 0.05 pt.

use std::io::Write;
use wordcraft_doc::table::Table;
use wordcraft_doc::{Block, Blocks, Document};
use wordcraft_layout::{autofit_widths, measure_table_columns, resolve_columns_probe};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(wordcraft_fonts::FontDb::init_test_fonts(), "init_test_fonts failed");

    let (doc, label): (Document, String) = if args.len() > 2 && args[1] == "--json" {
        let bytes = std::fs::read(&args[2]).unwrap_or_else(|e| panic!("read {}: {e}", args[2]));
        let d: Document = serde_json::from_slice(&bytes).expect("parse Document JSON");
        (d, args[2].clone())
    } else if args.len() > 1 {
        let bytes = std::fs::read(&args[1]).unwrap_or_else(|e| panic!("read {}: {e}", args[1]));
        let ext = args[1].rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let d = if ext == "doc" || ext == "dot" {
            wordcraft_docbin::read(&bytes).expect("doc (Word 97-2003) read")
        } else {
            wordcraft_docx::read(&bytes).expect("docx read")
        };
        let out = format!("{}.doc.json", args[1]);
        serde_json::to_writer_pretty(std::fs::File::create(&out).expect("create json out"), &d)
            .expect("serialize Document");
        (d, args[1].clone())
    } else {
        eprintln!("usage: table_dump <file.docx> | table_dump --json <doc.json>");
        std::process::exit(2);
    };

    let sec = &doc.last_section;
    let avail = sec.page_w - sec.margin_left - sec.margin_right - sec.gutter;

    let mut tables: Vec<serde_json::Value> = Vec::new();
    collect_tables(&doc, &doc.body, 0, &mut tables, avail);

    let report = serde_json::json!({
        "source": label,
        "tool": env!("CARGO_PKG_VERSION"),
        "avail_pt": avail,
        "page": { "w": sec.page_w, "h": sec.page_h, "ml": sec.margin_left, "mr": sec.margin_right, "gutter": sec.gutter },
        "tables": tables,
    });
    std::io::stdout()
        .write_all(serde_json::to_string_pretty(&report).expect("report json").as_bytes())
        .expect("write report");
    println!();
}

fn collect_tables(doc: &Document, blocks: &Blocks, depth: usize, out: &mut Vec<serde_json::Value>, avail: f32) {
    for b in blocks {
        if let Block::Table(t) = &**b {
            out.push(probe_table(doc, t, depth, avail));
            for row in &t.rows {
                for cell in &row.cells {
                    collect_tables(doc, &cell.blocks, depth + 1, out, avail);
                }
            }
        }
    }
}

fn probe_table(doc: &Document, t: &Table, depth: usize, avail: f32) -> serde_json::Value {
    let measured = measure_table_columns(doc, t);
    let probe = resolve_columns_probe(t, &measured, avail);
    let budgets = [avail, 400.0];
    let autofit: Vec<(f32, Vec<f32>)> = budgets.iter().map(|&a| (a, autofit_widths(&measured, a))).collect();

    serde_json::json!({
        "depth": depth,
        "table_props": serde_json::to_value(&t.props).expect("table props"),
        "grid": t.grid,
        "grid_sum_pt": t.grid.iter().sum::<f32>(),
        "rows": t.rows.iter().map(|r| serde_json::json!({
            "row_props": serde_json::to_value(&r.props).expect("row props"),
            "cells": r.cells.iter().map(|c| serde_json::to_value(&c.props).expect("cell props")).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "measured_min_max": measured,
        "autofit_at": autofit.into_iter().map(|(a, w)| serde_json::json!({ "avail": a, "widths": w })).collect::<Vec<_>>(),
        "resolved": {
            "preferred_grid_after_cell_widths": probe.0,
            "after_table_target": probe.1,
            "final": probe.2,
            "final_sum": probe.2.iter().sum::<f32>(),
            "tcw_accepted": probe.3,
        }
    })
}
