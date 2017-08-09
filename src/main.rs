extern crate gtk;
extern crate rusqlite;
extern crate time;

use time::{Timespec, strftime};

use rusqlite::Connection;

use gtk::prelude::*;
use gtk::{Builder, Window, Label, ListStore, TreeView};
use gtk::{TreeViewColumn, CellRendererText, TreeModelFilter};
use gtk::{Button, Entry};

const UI_GLADE: &'static str = include_str!("ui.glade");

struct Tape {
    id: u32,
    title: String,
    tape: String,
    ts: Timespec,
}

fn main() {
    let args: Vec<_> = ::std::env::args().collect();

    if args.len() < 2 {
        panic!("Usage: cassettes <path-to-tapes.db>");
    }

    let db_path = &args[1];

    let db = Connection::open(db_path).unwrap();

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let builder = Builder::new_from_string(UI_GLADE);

    let main_window: Window = builder.get_object("main_window").unwrap();

    main_window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let edit_button: Button = builder.get_object("edit_button").unwrap();
    edit_button.set_sensitive(false);

    let delete_button: Button = builder.get_object("delete_button").unwrap();
    delete_button.set_sensitive(false);

    let clear_button: Button = builder.get_object("clear_button").unwrap();
    clear_button.set_sensitive(false);

    let status_label: Label = builder.get_object("status_label").unwrap();

    let tape_treeview: TreeView = builder.get_object("tape_treeview").unwrap();

    let search_entry: Entry = builder.get_object("search_entry").unwrap();

    fn append_column(tree: &TreeView,
                     id: i32,
                     kind: &str,
                     title: &str,
                     visible: bool) {
        let column = TreeViewColumn::new();
        let cell = CellRendererText::new();

        column.set_title(title);
        column.set_sort_column_id(id);
        column.set_visible(visible);

        column.pack_start(&cell, true);
        // Association of the view's column with the model's `id` column.
        column.add_attribute(&cell, kind, id);
        tree.append_column(&column);
    }

    append_column(&tape_treeview, 0, "text", "ID", false);
    append_column(&tape_treeview, 1, "text", "Titre", true);
    append_column(&tape_treeview, 2, "text", "Cassette", true);
    append_column(&tape_treeview, 3, "text", "Ajouté", true);

    let tape_model = ListStore::new(&[u32::static_type(),
                                      String::static_type(),
                                      String::static_type(),
                                      String::static_type()]);

    let tape_model_filter = TreeModelFilter::new(&tape_model, None);

    let entry = search_entry.clone();

    tape_model_filter.set_visible_func(move |model, iter| {
        let search = entry.get_text().unwrap_or(String::new());

        // Make the search case-insensitive
        let search = search.to_uppercase();

        if search.is_empty() {
            // No filter
            true
        } else {
            let title: String = model.get_value(iter, 1).get().unwrap();
            let tape: String =  model.get_value(iter, 2).get().unwrap();

            title.to_uppercase().contains(&search) ||
                tape.to_uppercase().contains(&search)
        }
    });

    tape_treeview.set_model(Some(&tape_model_filter));

    let tapes = load_db(db);

    for tape in &tapes {
        let tm = time::at(tape.ts);
        let date = strftime("%Y-%m-%d %H:%M:%S", &tm).unwrap();

        tape_model.insert_with_values(None, &[0, 1, 2, 3],
                                      &[&tape.id,
                                        &tape.title,
                                        &tape.tape,
                                        &date]);
    }

    let status_text =
        format!("<span foreground=\"green\">{}</span> films référencés",
                tapes.len());

    status_label.set_markup(&status_text);

    tape_treeview.connect_cursor_changed(move |tree_view| {
        let selection = tree_view.get_selection();

        let entry_selected =
            if let Some(_) = selection.get_selected() {
                true
            } else {
                false
            };

        edit_button.set_sensitive(entry_selected);
        delete_button.set_sensitive(entry_selected);
    });

    let entry = search_entry.clone();
    clear_button.connect_clicked(move |_| {
        entry.set_text("");
    });

    search_entry.connect_changed(move |entry| {
        let text = entry.get_text().unwrap_or(String::new());

        tape_model_filter.refilter();

        if text.is_empty() {
            clear_button.set_sensitive(false);
        } else {
            clear_button.set_sensitive(true);
        }
    });

    main_window.show_all();

    gtk::main();
}

fn load_db(db: Connection) -> Vec<Tape> {
    let mut stmt =
        db.prepare("SELECT id, title, tape, timestamp \
                    FROM tapes ORDER BY id ASC").unwrap();

    let tapes = stmt.query_map(&[], |row| {
        Tape {
            id: row.get(0),
            title: row.get(1),
            tape: row.get(2),
            ts: row.get(3)
        }
    }).unwrap()
        .map(|t| t.unwrap())
        .collect();

    tapes
}
