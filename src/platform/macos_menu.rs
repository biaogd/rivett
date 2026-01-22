use std::sync::atomic::{AtomicBool, Ordering};

use objc2::define_class;
use objc2::rc::Retained;
use objc2::{msg_send, sel, MainThreadOnly};
use objc2_foundation::{MainThreadMarker, NSObject, NSString};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};

static SETTINGS_REQUESTED: AtomicBool = AtomicBool::new(false);
static MENU_SETUP_REQUESTED: AtomicBool = AtomicBool::new(false);
static MENU_INSTALLED: AtomicBool = AtomicBool::new(false);
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    struct MenuHandler;

    impl MenuHandler {
        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _item: Option<&NSMenuItem>) {
            SETTINGS_REQUESTED.store(true, Ordering::SeqCst);
        }
    }
);

impl MenuHandler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm);
        unsafe { msg_send![this, init] }
    }
}

pub fn setup() {
    MENU_SETUP_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn maybe_install() {
    if !MENU_SETUP_REQUESTED.load(Ordering::SeqCst) || MENU_INSTALLED.load(Ordering::SeqCst) {
        return;
    }

    let mtm = match MainThreadMarker::new() {
        Some(mtm) => mtm,
        None => return,
    };

    let handler = MenuHandler::new(mtm);

    let app = NSApplication::sharedApplication(mtm);
    let settings_title = NSString::from_str("Settings...");
    let settings_key = NSString::from_str(",");
    let quit_title = NSString::from_str("Quit SSH GUI");
    let quit_key = NSString::from_str("q");

    let Some(main_menu) = app.mainMenu() else {
        return;
    };

    let app_menu_item = main_menu.itemAtIndex(0).unwrap_or_else(|| {
        let empty_title = NSString::from_str("");
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &empty_title,
                None,
                &empty_title,
            )
        };
        main_menu.addItem(&item);
        item
    });

    let app_menu = if let Some(submenu) = app_menu_item.submenu() {
        submenu
    } else {
        let app_menu_title = NSString::from_str("Application");
        let app_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &app_menu_title);
        app_menu_item.setSubmenu(Some(&app_menu));
        app_menu
    };

    if app_menu.indexOfItemWithTitle(&settings_title) < 0 {
        let settings_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &settings_title,
                Some(sel!(openSettings:)),
                &settings_key,
            )
        };
        unsafe {
            settings_item.setTarget(Some(&*handler));
        }

        let insert_at = app_menu.indexOfItemWithTitle(&quit_title);
        if insert_at >= 0 {
            app_menu.insertItem_atIndex(&settings_item, insert_at);
            app_menu.insertItem_atIndex(&NSMenuItem::separatorItem(mtm), insert_at + 1);
        } else {
            app_menu.addItem(&settings_item);
            app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        }
    }

    if app_menu.indexOfItemWithTitle(&quit_title) < 0 {
        let quit_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &quit_title,
                Some(sel!(terminate:)),
                &quit_key,
            )
        };
        unsafe {
            quit_item.setTarget(Some(&*app));
        }
        app_menu.addItem(&quit_item);
    }

    std::mem::forget(handler);
    MENU_INSTALLED.store(true, Ordering::SeqCst);
}

pub fn take_settings_request() -> bool {
    SETTINGS_REQUESTED.swap(false, Ordering::SeqCst)
}
