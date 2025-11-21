// https://github.com/toml-rs/toml/blob/v0.23.7/crates/toml_edit/src/ser/pretty.rs
use toml_edit::{Array, DocumentMut, Item, Table, Value, visit_mut};

pub(crate) struct Pretty {
    in_value: bool,
    format_tables: bool,
}

impl Pretty {
    pub(crate) fn new(format_tables: bool) -> Self {
        Self {
            in_value: false,
            format_tables,
        }
    }
}

fn make_item(node: &mut Item) {
    *node = std::mem::take(node)
        .into_table()
        .map_or_else(|i| i, Item::Table)
        .into_array_of_tables()
        .map_or_else(|i| i, Item::ArrayOfTables);
}

impl visit_mut::VisitMut for Pretty {
    fn visit_document_mut(&mut self, node: &mut DocumentMut) {
        visit_mut::visit_document_mut(self, node);
    }

    fn visit_item_mut(&mut self, node: &mut Item) {
        if !self.in_value && self.format_tables {
            make_item(node);
        }

        visit_mut::visit_item_mut(self, node);
    }

    fn visit_table_mut(&mut self, node: &mut Table) {
        node.decor_mut().clear();

        // Empty tables could be semantically meaningful, so make sure they are not implicit
        if !node.is_empty() {
            node.set_implicit(true);
        }

        visit_mut::visit_table_mut(self, node);
    }

    fn visit_array_mut(&mut self, node: &mut Array) {
        visit_mut::visit_array_mut(self, node);

        if (0..=1).contains(&node.len()) {
            node.set_trailing("");
            node.set_trailing_comma(false);
        } else {
            for item in node.iter_mut() {
                item.decor_mut().set_prefix("\n    ");
            }
            node.set_trailing("\n");
            node.set_trailing_comma(true);
        }
    }

    fn visit_value_mut(&mut self, node: &mut Value) {
        node.decor_mut().clear();

        let old_in_value = self.in_value;
        self.in_value = true;
        visit_mut::visit_value_mut(self, node);
        self.in_value = old_in_value;
    }
}
