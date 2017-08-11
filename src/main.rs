extern crate gtk;
extern crate rusqlite;
extern crate time;

use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;

use time::{Timespec, strftime};

use rusqlite::Connection;

use gtk::prelude::*;
use gtk::{Builder, Window, Label, ListStore, TreeView};
use gtk::{TreeViewColumn, CellRendererText, TreeModelFilter};
use gtk::{Button, Entry, Dialog};

const UI_GLADE: &'static str = include_str!("ui.glade");

struct Tape {
    id: u32,
    title: String,
    tape: String,
    ts: Timespec,
}

struct Context {
    db: Connection,
    tapes: Vec<Tape>,
    main_window: Window,
    search_entry: Entry,
    add_button: Button,
    edit_button: Button,
    delete_button: Button,
    clear_button: Button,
    status_label: Label,
    tape_treeview: TreeView,
    tape_model: ListStore,
    tape_model_filter: TreeModelFilter,
}

impl Context {
    fn new(db_path: &Path) -> Context {
        let db = Connection::open(db_path).unwrap();

        let builder = Builder::new_from_string(UI_GLADE);

        let tape_model = ListStore::new(&[u32::static_type(),
                                          String::static_type(),
                                          String::static_type(),
                                          String::static_type()]);

        let tape_model_filter = TreeModelFilter::new(&tape_model, None);

        let mut context = Context {
            db: db,
            tapes: Vec::new(),
            main_window: builder.get_object("main_window").unwrap(),
            search_entry: builder.get_object("search_entry").unwrap(),
            add_button: builder.get_object("add_button").unwrap(),
            edit_button: builder.get_object("edit_button").unwrap(),
            delete_button: builder.get_object("delete_button").unwrap(),
            clear_button: builder.get_object("clear_button").unwrap(),
            status_label: builder.get_object("status_label").unwrap(),
            tape_treeview: builder.get_object("tape_treeview").unwrap(),
            tape_model: tape_model,
            tape_model_filter: tape_model_filter,

        };

        context.tape_treeview.set_model(Some(&context.tape_model_filter));

        context.load_tapes();

        context
    }

    fn load_tapes(&mut self) {
        let mut stmt =
            self.db.prepare("SELECT id, title, tape, timestamp \
                             FROM tapes ORDER BY id ASC").unwrap();

        self.tapes = stmt.query_map(&[], |row| {
            Tape {
                id: row.get(0),
                title: row.get(1),
                tape: row.get(2),
                ts: row.get(3)
            }
        }).unwrap()
            .map(|t| t.unwrap())
            .collect();

        for tape in &self.tapes {
            let tm = time::at(tape.ts);
            let date = strftime("%Y-%m-%d %H:%M:%S", &tm).unwrap();

            self.tape_model.insert_with_values(None, &[0, 1, 2, 3],
                                               &[&tape.id,
                                                 &tape.title,
                                                 &tape.tape,
                                                 &date]);
        }
    }

    fn treeview_add_column(&self, id: i32, title: &str, visible: bool) {
        let column = TreeViewColumn::new();
        let cell = CellRendererText::new();

        column.set_title(title);
        column.set_sort_column_id(id);
        column.set_visible(visible);

        column.pack_start(&cell, true);
        // Association of the view's column with the model's `id` column.
        column.add_attribute(&cell, "text", id);
        self.tape_treeview.append_column(&column);
    }
}

fn main() {
    let args: Vec<_> = ::std::env::args().collect();

    if args.len() < 2 {
        panic!("Usage: cassettes <path-to-tapes.db>");
    }

    let db_path = Path::new(&args[1]);

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let context = Rc::new(RefCell::new(Context::new(db_path)));

    ui_init(&context);

    gtk::main();
}

fn ui_init(context: &Rc<RefCell<Context>>) {
    let ctx = context.borrow();

    ctx.main_window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    ctx.edit_button.set_sensitive(false);
    ctx.delete_button.set_sensitive(false);
    ctx.clear_button.set_sensitive(false);

    ctx.treeview_add_column(0, "ID", false);
    ctx.treeview_add_column(1, "Titre", true);
    ctx.treeview_add_column(2, "Cassette", true);
    ctx.treeview_add_column(3, "Ajouté", true);

    let ctx_clone = context.clone();

    ctx.tape_model_filter.set_visible_func(move |model, iter| {
        let search_entry = &ctx_clone.borrow().search_entry;

        let search = search_entry.get_text().unwrap_or(String::new());

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

    let status_text =
        format!("<span foreground=\"green\">{}</span> films référencés",
                context.borrow().tapes.len());

    ctx.status_label.set_markup(&status_text);

    let ctx_clone = context.clone();

    ctx.tape_treeview.connect_cursor_changed(move |tree_view| {
        let selection = tree_view.get_selection();

        let entry_selected =
            if let Some(_) = selection.get_selected() {
                true
            } else {
                false
            };

        let ctx = ctx_clone.borrow();

        ctx.edit_button.set_sensitive(entry_selected);
        ctx.delete_button.set_sensitive(entry_selected);
    });

    let ctx_clone = context.clone();

    ctx.clear_button.connect_clicked(move |_| {
        ctx_clone.borrow().search_entry.set_text("");
    });

    let ctx_clone = context.clone();

    ctx.search_entry.connect_changed(move |entry| {
        let ctx = ctx_clone.borrow();

        let search = ctx.search_entry.get_text().unwrap_or(String::new());

        if search.is_empty() {
            ctx.clear_button.set_sensitive(false);
        } else {
            ctx.clear_button.set_sensitive(true);
        }

        ctx.tape_model_filter.refilter();
    });

    ctx.main_window.show_all();
}

