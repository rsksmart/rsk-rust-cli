use colored::Colorize;
use prettytable::{Cell, Row, Table};

pub struct TableBuilder {
    table: Table,
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TableBuilder {
    pub fn new() -> Self {
        Self {
            table: Table::new(),
        }
    }

    pub fn add_header(&mut self, headers: &[&str]) -> &mut Self {
        let row = Row::new(headers.iter().map(|h| Cell::new(h)).collect());
        self.table.set_titles(row);
        self
    }

    pub fn add_row(&mut self, cells: &[&str]) -> &mut Self {
        let row = Row::new(cells.iter().map(|c| Cell::new(c)).collect());
        self.table.add_row(row);
        self
    }

    pub fn print(&self) {
        let mut buf = Vec::new();
        self.table.print(&mut buf).expect("Failed to print table");
        if let Ok(output) = String::from_utf8(buf) {
            println!("\n{}", output);
        }
    }

    pub fn _print_error(&self, error: &str) {
        eprintln!("{}: {}", "Error".red().bold(), error);
    }
}
