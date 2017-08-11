extern crate gtk;
extern crate rusqlite;
extern crate time;

use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;

use time::strftime;

use rusqlite::Connection;

use gtk::prelude::*;
use gtk::{Builder, Window, Label, ListStore, TreeView};
use gtk::{TreeViewColumn, CellRendererText, TreeModelFilter};
use gtk::{Button, Entry, Dialog, ResponseType};

const UI_GLADE: &'static str = include_str!("ui.glade");

struct Tape {
    id: u32,
    title: String,
    tape: String,
    date: String,
}

struct Context {
    db: Connection,
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

    add_dialog: Dialog,
    add_add_button: Button,
    add_cancel_button: Button,
    add_title_entry: Entry,
    add_tape_entry: Entry,

    delete_dialog: Dialog,
    delete_delete_button: Button,
    delete_cancel_button: Button,
    delete_title_entry: Entry,
    delete_tape_entry: Entry,
    delete_date_entry: Entry,
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

        let context = Context {
            db: db,
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

            add_dialog: builder.get_object("add_dialog").unwrap(),
            add_add_button: builder.get_object("add_add_button").unwrap(),
            add_cancel_button: builder.get_object("add_cancel_button").unwrap(),
            add_title_entry: builder.get_object("add_title_entry").unwrap(),
            add_tape_entry: builder.get_object("add_tape_entry").unwrap(),

            delete_dialog: builder.get_object("delete_dialog").unwrap(),
            delete_delete_button: builder.get_object("delete_delete_button").unwrap(),
            delete_cancel_button: builder.get_object("delete_cancel_button").unwrap(),
            delete_title_entry: builder.get_object("delete_title_entry").unwrap(),
            delete_tape_entry: builder.get_object("delete_tape_entry").unwrap(),
            delete_date_entry: builder.get_object("delete_date_entry").unwrap(),
        };

        context.tape_treeview.set_model(Some(&context.tape_model_filter));

        context.load_tapes();

        context
    }

    fn load_tapes(&self) {
        self.tape_model.clear();

        let mut stmt =
            self.db.prepare("SELECT id, title, tape, timestamp \
                             FROM tapes ORDER BY id DESC").unwrap();

        let tapes: Vec<_> = stmt.query_map(&[], |row| {
            let ts = row.get(3);

            let tm = time::at(ts);
            let date = strftime("%Y-%m-%d %H:%M:%S", &tm).unwrap();

            Tape {
                id: row.get(0),
                title: row.get(1),
                tape: row.get(2),
                date: date,
            }
        }).unwrap()
            .map(|t| t.unwrap())
            .collect();

        for tape in &tapes {

            self.tape_model.insert_with_values(None, &[0, 1, 2, 3],
                                               &[&tape.id,
                                                 &tape.title,
                                                 &tape.tape,
                                                 &tape.date]);
        }

        let status_text =
            format!("<span foreground=\"green\">{}</span> films référencés",
                    tapes.len());

        self.status_label.set_markup(&status_text);
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

    fn check_add_filled(&self) {
        let title = self.add_title_entry.get_text().unwrap_or(String::new());
        let tape = self.add_tape_entry.get_text().unwrap_or(String::new());

        let filled = !title.is_empty() && !tape.is_empty();

        self.add_add_button.set_sensitive(filled);
    }

    fn do_add_tape(&self) {
        let title = self.add_title_entry.get_text().unwrap_or(String::new());
        let tape = self.add_tape_entry.get_text().unwrap_or(String::new());

        if title.is_empty() || tape.is_empty() {
            // Shouldn't happen
            return;
        }


        self.db.execute("INSERT INTO tapes (title, tape) \
                         VALUES (?1, ?2)",
                        &[&title, &tape]).unwrap();

        self.load_tapes();
    }

    fn do_delete_tape(&self, id: u32) {
        self.db.execute("DELETE FROM tapes WHERE id = ?1",
                        &[&id]).unwrap();

        self.load_tapes();
    }

    fn get_selection(&self) -> Option<Tape> {
        let selection = self.tape_treeview.get_selection();

        selection.get_selected().map(|(model, iter)| {
            Tape {
                id: model.get_value(&iter, 0).get().unwrap(),
                title: model.get_value(&iter, 1).get().unwrap(),
                tape: model.get_value(&iter, 2).get().unwrap(),
                date: model.get_value(&iter, 3).get().unwrap(),
            }
        })
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

        let search = entry.get_text().unwrap_or(String::new());

        if search.is_empty() {
            ctx.clear_button.set_sensitive(false);
        } else {
            ctx.clear_button.set_sensitive(true);
        }

        ctx.tape_model_filter.refilter();
    });

    let ctx_clone = context.clone();

    ctx.add_button.connect_clicked(move |_| {
        let context = &ctx_clone;
        let ctx = context.borrow();

        ctx.add_dialog.set_transient_for(&ctx.main_window);
        ctx.add_dialog.set_modal(true);

        ctx.add_add_button.set_sensitive(false);

        let ctx_clone = context.clone();

        ctx.add_cancel_button.connect_clicked(move |_| {
            let ctx = ctx_clone.borrow();

            ctx.add_dialog.response(ResponseType::Cancel.into());
        });


        let ctx_clone = context.clone();
        ctx.add_title_entry.connect_changed(move |_| {
            ctx_clone.borrow().check_add_filled();
        });

        let ctx_clone = context.clone();
        ctx.add_tape_entry.connect_changed(move |_| {
            ctx_clone.borrow().check_add_filled();
        });

        let ctx_clone = context.clone();

        ctx.add_add_button.connect_clicked(move |_| {
            let ctx = ctx_clone.borrow();

            ctx.add_dialog.response(ResponseType::Ok.into());
        });

        ctx.add_dialog.show_all();

        let result = ctx.add_dialog.run();

        ctx.add_dialog.hide();

        if result == ResponseType::Ok.into() {
            ctx.do_add_tape();
        }
    });

    let ctx_clone = context.clone();

    ctx.delete_button.connect_clicked(move |_| {
        let context = &ctx_clone;
        let ctx = context.borrow();

        ctx.delete_dialog.set_transient_for(&ctx.main_window);
        ctx.delete_dialog.set_modal(true);

        ctx.delete_title_entry.set_sensitive(false);
        ctx.delete_tape_entry.set_sensitive(false);
        ctx.delete_date_entry.set_sensitive(false);

        let selected = match ctx.get_selection() {
            Some(t) => t,
            None => return,
        };

        ctx.delete_title_entry.set_text(&selected.title);
        ctx.delete_tape_entry.set_text(&selected.tape);
        ctx.delete_date_entry.set_text(&selected.date);

        let ctx_clone = context.clone();

        ctx.delete_cancel_button.connect_clicked(move |_| {
            let ctx = ctx_clone.borrow();

            ctx.delete_dialog.response(ResponseType::Cancel.into());
        });


        // let ctx_clone = context.clone();
        // ctx.add_title_entry.connect_changed(move |_| {
        //     ctx_clone.borrow().check_add_filled();
        // });

        // let ctx_clone = context.clone();
        // ctx.add_tape_entry.connect_changed(move |_| {
        //     ctx_clone.borrow().check_add_filled();
        // });

        let ctx_clone = context.clone();

        ctx.delete_delete_button.connect_clicked(move |_| {
            let ctx = ctx_clone.borrow();

            ctx.delete_dialog.response(ResponseType::Ok.into());
        });

        ctx.delete_dialog.show_all();

        let result = ctx.delete_dialog.run();

        ctx.delete_dialog.hide();

        if result == ResponseType::Ok.into() {
            ctx.do_delete_tape(selected.id);
        }
    });

    ctx.main_window.show_all();
}
