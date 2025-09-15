========================
CODE SNIPPETS
========================
TITLE: Tauri Example: Adding Capabilities
DESCRIPTION: Shows how to add capabilities to a Tauri application during the setup phase. This example conditionally adds 'beta' and 'stable' capabilities based on Cargo features, using `include_str!` to load capability JSON files.

SOURCE: https://docs.rs/tauri/latest/src/tauri/lib

LANGUAGE: rust
CODE:
```
use tauri::Manager;

tauri::Builder::default()
  .setup(|app| {
    #[cfg(feature = "beta")]
    app.add_capability(include_str!("../capabilities/beta/cap.json"));

    #[cfg(feature = "stable")]
    app.add_capability(include_str!("../capabilities/stable/cap.json"));
    Ok(())
  });
```

----------------------------------------

TITLE: Rust: Tauri Builder Example - Listen to Event
DESCRIPTION: An example demonstrating how to set up an event listener for 'component-loaded' on a Tauri window during the application's setup phase. It logs a message when the event occurs.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/window/mod

LANGUAGE: rust
CODE:
```
use tauri::{Manager, Listener};

tauri::Builder::default()
  .setup(|app| {
    let window = app.get_window("main").unwrap();
    window.listen("component-loaded", move |event| {
      println!("window just loaded a component");
    });

    Ok(())
  });

```

----------------------------------------

TITLE: Rust: Tauri Builder Example - Listen to Event
DESCRIPTION: An example demonstrating how to set up an event listener for 'component-loaded' on a Tauri window during the application's setup phase. It logs a message when the event occurs.

SOURCE: https://docs.rs/tauri/latest/src/tauri/window/mod

LANGUAGE: rust
CODE:
```
use tauri::{Manager, Listener};

tauri::Builder::default()
  .setup(|app| {
    let window = app.get_window("main").unwrap();
    window.listen("component-loaded", move |event| {
      println!("window just loaded a component");
    });

    Ok(())
  });

```

----------------------------------------

TITLE: Create Webview Window in Setup Hook
DESCRIPTION: Demonstrates how to create a new webview window with a specific label and URL within the Tauri application's setup hook. This is a common pattern for initializing the main window or other essential windows when the application starts.

SOURCE: https://docs.rs/tauri/latest/src/tauri/webview/webview_window

LANGUAGE: rust
CODE:
```
tauri::Builder::default()
  .setup(|app| {
    let webview_window = tauri::WebviewWindowBuilder::new(app, "label", tauri::WebviewUrl::App("index.html".into()))
      .build()?;
    Ok(())
  });

```

----------------------------------------

TITLE: Setup Hook for Tauri App
DESCRIPTION: Defines the setup hook for a Tauri application. This function is executed once when the application starts. It allows for initialization tasks, such as setting window titles. Requires the `tauri` crate and the `Manager` trait.

SOURCE: https://docs.rs/tauri/latest/src/tauri/app

LANGUAGE: rust
CODE:
```
use tauri::Manager;
tauri::Builder::default()
  .setup(|app| {
    let main_window = app.get_webview_window("main").unwrap();
    main_window.set_title("Tauri!")?;
    Ok(())
  });

```

----------------------------------------

TITLE: Setup Hook for Tauri App
DESCRIPTION: Defines the setup hook for a Tauri application. This function is executed once when the application starts. It allows for initialization tasks, such as setting window titles. Requires the `tauri` crate and the `Manager` trait.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/app

LANGUAGE: rust
CODE:
```
use tauri::Manager;
tauri::Builder::default()
  .setup(|app| {
    let main_window = app.get_webview_window("main").unwrap();
    main_window.set_title("Tauri!")?;
    Ok(())
  });

```

----------------------------------------

TITLE: Rust: Tauri Builder Example - Listen and Unlisten
DESCRIPTION: An example demonstrating how to use `listen` and `unlisten` within a Tauri application's setup. It shows how to get a window, set up a listener for 'component-loaded', and then unlisten either within the handler or after the listener is set up.

SOURCE: https://docs.rs/tauri/latest/src/tauri/window/mod

LANGUAGE: rust
CODE:
```
use tauri::{Manager, Listener};

tauri::Builder::default()
  .setup(|app| {
    let window = app.get_window("main").unwrap();
    let window_ = window.clone();
    let handler = window.listen("component-loaded", move |event| {
      println!("window just loaded a component");

      // we no longer need to listen to the event
      // we also could have used `window.once` instead
      window_.unlisten(event.id());
    });

    // stop listening to the event when you do not need it anymore
    window.unlisten(handler);

    Ok(())
  });

```

----------------------------------------

TITLE: Rust: Tauri Builder Example - Listen and Unlisten
DESCRIPTION: An example demonstrating how to use `listen` and `unlisten` within a Tauri application's setup. It shows how to get a window, set up a listener for 'component-loaded', and then unlisten either within the handler or after the listener is set up.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/window/mod

LANGUAGE: rust
CODE:
```
use tauri::{Manager, Listener};

tauri::Builder::default()
  .setup(|app| {
    let window = app.get_window("main").unwrap();
    let window_ = window.clone();
    let handler = window.listen("component-loaded", move |event| {
      println!("window just loaded a component");

      // we no longer need to listen to the event
      // we also could have used `window.once` instead
      window_.unlisten(event.id());
    });

    // stop listening to the event when you do not need it anymore
    window.unlisten(handler);

    Ok(())
  });

```

----------------------------------------

TITLE: Setup Tauri Application in Rust
DESCRIPTION: Configures and initializes the Tauri application, including creating initial windows based on configuration, setting up assets, and executing a user-provided setup function.

SOURCE: https://docs.rs/tauri/latest/src/tauri/app

LANGUAGE: rust
CODE:
```
#[cfg_attr(feature = "tracing", tracing::instrument(name = "app::setup"))]
fn setup<R: Runtime>(app: &mut App<R>) -> crate::Result<()> {
  app.ran_setup = true;

  for window_config in app.config().app.windows.iter().filter(|w| w.create) {
    WebviewWindowBuilder::from_config(app.handle(), window_config)?.build()?;
  }

  app.manager.assets.setup(app);

  if let Some(setup) = app.setup.take() {
    (setup)(app).map_err(|e| crate::Error::Setup(e.into()))?;
  }

  Ok(())
}
```

----------------------------------------

TITLE: Tauri Example: Emitting and Listening to Events
DESCRIPTION: Demonstrates how to emit an event from a Tauri command and listen for it within the application setup. The `synchronize` command emits a 'synchronized' event, and the `setup` closure registers a handler to print a message when this event is received.

SOURCE: https://docs.rs/tauri/latest/src/tauri/lib

LANGUAGE: rust
CODE:
```
use tauri::{Manager, Listener, Emitter};

#[tauri::command]
fn synchronize(window: tauri::Window) {
  // emits the synchronized event to all windows
  window.emit("synchronized", ());
}

tauri::Builder::default()
  .setup(|app| {
    app.listen("synchronized", |event| {
      println!("app is in sync");
    });
    Ok(())
  })
  .invoke_handler(tauri::generate_handler![synchronize]);
```

----------------------------------------

TITLE: Setup Tauri Application in Rust
DESCRIPTION: Configures and initializes the Tauri application, including creating initial windows based on configuration, setting up assets, and executing a user-provided setup function.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/app

LANGUAGE: rust
CODE:
```
#[cfg_attr(feature = "tracing", tracing::instrument(name = "app::setup"))]
fn setup<R: Runtime>(app: &mut App<R>) -> crate::Result<()> {
  app.ran_setup = true;

  for window_config in app.config().app.windows.iter().filter(|w| w.create) {
    WebviewWindowBuilder::from_config(app.handle(), window_config)?.build()?;
  }

  app.manager.assets.setup(app);

  if let Some(setup) = app.setup.take() {
    (setup)(app).map_err(|e| crate::Error::Setup(e.into()))?;
  }

  Ok(())
}
```

----------------------------------------

TITLE: Example: Tauri Page Load Event Handling
DESCRIPTION: An example illustrating the use of the `on_page_load` method in Tauri. It sets up a handler that logs messages when a page starts or finishes loading, printing the URL of the loaded page. This allows for monitoring the loading progress of web content within the webview.

SOURCE: https://docs.rs/tauri/latest/src/tauri/webview/mod

LANGUAGE: rust
CODE:
```
use tauri::{
  utils::config::{Csp, CspDirectiveSources, WebviewUrl},
  window::WindowBuilder,
  webview::{PageLoadEvent, WebviewBuilder},
};
use http::header::HeaderValue;
use std::collections::HashMap;
tauri::Builder::default()
  .setup(|app| {
    let window = tauri::window::WindowBuilder::new(app, "label").build()?;
    let webview_builder = WebviewBuilder::new("core", WebviewUrl::App("index.html".into()))
      .on_page_load(|webview, payload| {
        match payload.event() {
          PageLoadEvent::Started => {
            println!("{} finished loading", payload.url());
          }
          PageLoadEvent::Finished => {
            println!("{} finished loading", payload.url());
          }
        }
      });
    let webview = window.add_child(webview_builder, tauri::LogicalPosition::new(0, 0), window.inner_size().unwrap())?; 
    Ok(())
  });

```

----------------------------------------

TITLE: Example: Tauri Download Event Handling
DESCRIPTION: An example demonstrating how to use the `on_download` method in Tauri. It configures a handler that prints download information and sets a custom destination path for requested downloads. The handler returns `true` to allow the download to proceed.

SOURCE: https://docs.rs/tauri/latest/src/tauri/webview/mod

LANGUAGE: rust
CODE:
```
use tauri::{
  utils::config::{Csp, CspDirectiveSources, WebviewUrl},
  window::WindowBuilder,
  webview::{DownloadEvent, WebviewBuilder},
};

tauri::Builder::default()
  .setup(|app| {
    let window = WindowBuilder::new(app, "label").build()?;
    let webview_builder = WebviewBuilder::new("core", WebviewUrl::App("index.html".into()))
      .on_download(|webview, event| {
        match event {
          DownloadEvent::Requested { url, destination } => {
            println!("downloading {}", url);
            *destination = "/home/tauri/target/path".into();
          }
          DownloadEvent::Finished { url, path, success } => {
            println!("downloaded {} to {:?}, success: {}", url, path, success);
          }
          _ => (),
        }
        // let the download start
        true
      });

    let webview = window.add_child(webview_builder, tauri::LogicalPosition::new(0, 0), window.inner_size().unwrap())?; 
    Ok(())
  });

```

----------------------------------------

TITLE: Rust: Accessing and Managing Tauri State with Examples
DESCRIPTION: Provides examples of managing and accessing different types of state (integers and strings) within a Tauri application setup. It shows how to use `app.manage()` and `app.state()` for state interaction.

SOURCE: https://docs.rs/tauri/latest/src/tauri/lib

LANGUAGE: rust
CODE:
```
use tauri::{Manager, State};

struct MyInt(isize);
struct MyString(String);

#[tauri::command]
fn int_command(state: State<MyInt>) -> String {
    format!("The stateful int is: {}", state.0)
}

#[tauri::command]
fn string_command<'r>(state: State<'r, MyString>) {
    println!("state: {}", state.inner().0);
}

tauri::Builder::default()
  .setup(|app| {
    app.manage(MyInt(0));
    app.manage(MyString("tauri".into()));
    // `MyInt` is already managed, so `manage()` returns false
    assert!(!app.manage(MyInt(1)));
    // read the `MyInt` managed state with the turbofish syntax
    let int = app.state::<MyInt>();
    assert_eq!(int.0, 0);
    // read the `MyString` managed state with the `State` guard
    let val: State<MyString> = app.state();
    assert_eq!(val.0, "tauri");
    Ok(())
  })
  .invoke_handler(tauri::generate_handler![int_command, string_command])
  // on an actual app, remove the string argument
  .run(tauri::generate_context!("test/fixture/src-tauri/tauri.conf.json"))
  .expect("error while running tauri application");
```

----------------------------------------

TITLE: Run Tauri Application
DESCRIPTION: Starts the Tauri application's event loop. This function is blocking and exits the process directly upon completion. It handles application setup and allows for custom event callbacks. Panics if the setup function fails.

SOURCE: https://docs.rs/tauri/latest/src/tauri/app

LANGUAGE: rust
CODE:
```
pub fn run<F: FnMut(&AppHandle<R>, RunEvent) + 'static>(mut self, callback: F) {
    self.handle.event_loop.lock().unwrap().main_thread_id = std::thread::current().id();

    self
      .runtime
      .take()
      .unwrap()
      .run(self.make_run_event_loop_callback(callback));
}
```

----------------------------------------

TITLE: Create Webview Window in Setup Hook - Rust
DESCRIPTION: Demonstrates how to create a new webview window with a specific label and URL during the Tauri application's setup phase. This is a common pattern for initializing the main window or other essential UI components.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/webview/webview_window

LANGUAGE: rust
CODE:
```
tauri::Builder::default()
  .setup(|app| {
    let webview_window = tauri::WebviewWindowBuilder::new(app, "label", tauri::WebviewUrl::App("index.html".into()))
      .build()?;
    Ok(())
  });

```

----------------------------------------

TITLE: Example: Setting Window Effects
DESCRIPTION: Demonstrates how to set window effects using `EffectsBuilder`. This example shows how to apply a popover effect with specific color and radius.

SOURCE: https://docs.rs/tauri/latest/src/tauri/window/mod

LANGUAGE: rust
CODE:
```
use tauri::{Manager, window::{Color, Effect, EffectState, EffectsBuilder}};
tauri::Builder::default()
  .setup(|app| {
    let window = app.get_window("main").unwrap();
    window.set_effects(
      EffectsBuilder::new()
        .effect(Effect::Popover)
        .state(EffectState::Active)
        .radius(5.)
        .color(Color(0, 0, 0, 255))
        .build(),
    )?;
    Ok(())
  });

```

----------------------------------------

TITLE: Handle Menu Events in Tauri App
DESCRIPTION: Example demonstrating how to set up a menu and handle its events within a Tauri application's setup function. It creates a 'Save' menu item and defines a callback for when it's clicked.

SOURCE: https://docs.rs/tauri/latest/src/tauri/webview/webview_window

LANGUAGE: rust
CODE:
```
use tauri::menu::{Menu, Submenu, MenuItem};
  tauri::Builder::default()
    .setup(|app| {
      let handle = app.handle();
      let save_menu_item = MenuItem::new(handle, "Save", true, None::<&str>)?;
      let menu = Menu::with_items(handle, &[
        &Submenu::with_items(handle, "File", true, &[
          &save_menu_item,
        ])?,
      ])?;
      let webview_window = tauri::WebviewWindowBuilder::new(app, "editor", tauri::WebviewUrl::App("index.html".into()))
        .menu(menu)
        .on_menu_event(move |window, event| {
          if event.id == save_menu_item.id() {
            // save menu item
          }
        })
        .build()
        .unwrap();

      Ok(())
    });
```

----------------------------------------

TITLE: Rust Learning Resources
DESCRIPTION: Provides a list of official resources for learning Rust and its ecosystem. This includes the main Rust website, 'The Book' for learning the language, the Standard Library API Reference, Rust by Example, and guides for Cargo and Clippy.

SOURCE: https://docs.rs/tauri/2.8.5/settings

LANGUAGE: Text
CODE:
```
Rust website
The Book
Standard Library API Reference
Rust by Example
The Cargo Guide
Clippy Documentation
```

----------------------------------------

TITLE: Tauri Application Setup and Event Handling
DESCRIPTION: Internal callback function for the Tauri runtime event loop. It manages application setup, processes incoming events (Ready, Exit, etc.), invokes user-defined callbacks, and handles application cleanup and restart logic.

SOURCE: https://docs.rs/tauri/latest/src/tauri/app

LANGUAGE: rust
CODE:
```
fn make_run_event_loop_callback<F: FnMut(&AppHandle<R>, RunEvent) + 'static>(
    mut self,
    mut callback: F,
) -> impl FnMut(RuntimeRunEvent<EventLoopMessage>) {
    let app_handle = self.handle().clone();
    let manager = self.manager.clone();

    move |event| match event {
      RuntimeRunEvent::Ready => {
        if let Err(e) = setup(&mut self) {
          panic!("Failed to setup app: {e}");
        }
        let event = on_event_loop_event(&app_handle, RuntimeRunEvent::Ready, &manager);
        callback(&app_handle, event);
      }
      RuntimeRunEvent::Exit => {
        let event = on_event_loop_event(&app_handle, RuntimeRunEvent::Exit, &manager);
        callback(&app_handle, event);
        app_handle.cleanup_before_exit();
        if self.manager.restart_on_exit.load(atomic::Ordering::Relaxed) {
          crate::process::restart(&self.env());
        }
      }
      _ => {
        let event = on_event_loop_event(&app_handle, event, &manager);
        callback(&app_handle, event);
      }
    }
}
```

----------------------------------------

TITLE: Tauri App Setup and Event Handling
DESCRIPTION: This Rust code defines the core structures and event types for a Tauri application. It includes definitions for window events, setup hooks, and various event listeners for managing application behavior, window interactions, and webview events.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/app

LANGUAGE: rust
CODE:
```
use crate::\{
  image::Image,
  ipc::\{
    channel::ChannelDataIpcQueue,
    CallbackFn,
    CommandArg,
    CommandItem,
    Invoke,
    InvokeError,
    InvokeHandler,
    InvokeResponseBody,
  \},
  manager::\{webview::UriSchemeProtocol, AppManager, Asset\},
  plugin::\{Plugin, PluginStore\},
  resources::ResourceTable,
  runtime::\{
    window::\{WebviewEvent as RuntimeWebviewEvent, WindowEvent as RuntimeWindowEvent\},
    ExitRequestedEventAction,
    RunEvent as RuntimeRunEvent,
  \},
  sealed::\{ManagerBase, RuntimeOrDispatch\},
  utils::\{config::Config, Env\},
  webview::PageLoadPayload,
  Context,
  DeviceEventFilter,
  Emitter,
  EventLoopMessage,
  EventName,
  Listener,
  Manager,
  Monitor,
  Runtime,
  Scopes,
  StateManager,
  Theme,
  Webview,
  WebviewWindowBuilder,
  Window,
\};

#[cfg(desktop)]
use crate::menu::Menu;
#[cfg(all(desktop, feature = "tray-icon"))]
use crate::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent, TrayIconId};
use raw_window_handle::HasDisplayHandle;
use serialize_to_javascript::{default_template, DefaultTemplate, Template};
use tauri_macros::default_runtime;
#[cfg(desktop)]
use tauri_runtime::EventLoopProxy;
use tauri_runtime::\{
  dpi::\{PhysicalPosition, PhysicalSize\},
  window::DragDropEvent,
  RuntimeInitArgs,
\};
use tauri_utils::assets::AssetsIter;
use tauri_utils::PackageInfo;

use std::\{
  borrow::Cow,
  collections::HashMap,
  fmt,
  sync::\{
    atomic,
    mpsc::Sender,
    Arc,
    Mutex,
    MutexGuard,
  \},
  thread::ThreadId,
  time::Duration,
\};

use crate::\{event::EventId, runtime::RuntimeHandle, Event, EventTarget\};

#[cfg(target_os = "macos")]
use crate::ActivationPolicy;

pub(crate) mod plugin;

#[cfg(desktop)]
pub(crate) type GlobalMenuEventListener<T> = Box<dyn Fn(&T, crate::menu::MenuEvent) + Send + Sync>;
#[cfg(all(desktop, feature = "tray-icon"))]
pub(crate) type GlobalTrayIconEventListener<T> =
  Box<dyn Fn(&T, crate::tray::TrayIconEvent) + Send + Sync>;
pub(crate) type GlobalWindowEventListener<R> = Box<dyn Fn(&Window<R>, &WindowEvent) + Send + Sync>;
pub(crate) type GlobalWebviewEventListener<R> =
  Box<dyn Fn(&Webview<R>, &WebviewEvent) + Send + Sync>;
/// A closure that is run when the Tauri application is setting up.
pub type SetupHook<R> =
  Box<dyn FnOnce(&mut App<R>) -> std::result::Result<(), Box<dyn std::error::Error>> + Send>;
/// A closure that is run every time a page starts or finishes loading.
pub type OnPageLoad<R> = dyn Fn(&Webview<R>, &PageLoadPayload<'_>) + Send + Sync + 'static;
pub type ChannelInterceptor<R> =
  Box<dyn Fn(&Webview<R>, CallbackFn, usize, &InvokeResponseBody) -> bool + Send + Sync + 'static>;

/// The exit code on [`RunEvent::ExitRequested`] when [`AppHandle#method.restart`] is called.
pub const RESTART_EXIT_CODE: i32 = i32::MAX;

/// Api exposed on the `ExitRequested` event.
#[derive(Debug, Clone)]
pub struct ExitRequestApi {
  tx: Sender<ExitRequestedEventAction>,
  code: Option<i32>,
}

impl ExitRequestApi {
  /// Prevents the app from exiting.
  ///
  /// **Note:** This is ignored when using [`AppHandle#method.restart`].
  pub fn prevent_exit(&self) {
    if self.code != Some(RESTART_EXIT_CODE) {
      self.tx.send(ExitRequestedEventAction::Prevent).unwrap();
    }
  }
}

/// Api exposed on the `CloseRequested` event.
#[derive(Debug, Clone)]
pub struct CloseRequestApi(Sender<bool>);

impl CloseRequestApi {
  /// Prevents the window from being closed.
  pub fn prevent_close(&self) {
    self.0.send(true).unwrap();
  }
}

/// An event from a window.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum WindowEvent {
  /// The size of the window has changed. Contains the client area's new dimensions.
  Resized(PhysicalSize<u32>),
  /// The position of the window has changed. Contains the window's new position.
  Moved(PhysicalPosition<i32>),
  /// The window has been requested to close.
  #[non_exhaustive]
  CloseRequested {
    /// An API modify the behavior of the close requested event.
    api: CloseRequestApi,
  },
  /// The window has been destroyed.
  Destroyed,
  /// The window gained or lost focus.
  ///
  /// The parameter is true if the window has gained focus, and false if it has lost focus.
  Focused(bool),
  /// The window's scale factor has changed.
  ///
  /// The following user actions can cause DPI changes:
  ///
  /// - Changing the display's resolution.
  /// - Changing the display's scale factor (e.g. in Control Panel on Windows).
  /// - Moving the window to a display with a different scale factor.
  #[non_exhaustive]
  ScaleFactorChanged {
    /// The new scale factor.
    scale_factor: f64,
  },
}

```

----------------------------------------

TITLE: Create Webview in Setup Hook - Tauri
DESCRIPTION: Demonstrates how to create a webview within the Tauri application's setup hook. It initializes a WebviewBuilder with a label and an app-relative URL, then adds it as a child to a window.

SOURCE: https://docs.rs/tauri/latest/src/tauri/webview/mod

LANGUAGE: rust
CODE:
```
tauri::Builder::default()
  .setup(|app| {
    let window = tauri::window::WindowBuilder::new(app, "label").build()?;
    let webview_builder = tauri::webview::WebviewBuilder::new("label", tauri::WebviewUrl::App("index.html".into()));
    let webview = window.add_child(webview_builder, tauri::LogicalPosition::new(0, 0), window.inner_size().unwrap());
    Ok(())
  });
```

----------------------------------------

TITLE: Tauri App Setup and Event Handling
DESCRIPTION: This Rust code defines the core structures and event types for a Tauri application. It includes definitions for window events, setup hooks, and various event listeners for managing application behavior, window interactions, and webview events.

SOURCE: https://docs.rs/tauri/latest/src/tauri/app

LANGUAGE: rust
CODE:
```
use crate::\{
  image::Image,
  ipc::\{
    channel::ChannelDataIpcQueue,
    CallbackFn,
    CommandArg,
    CommandItem,
    Invoke,
    InvokeError,
    InvokeHandler,
    InvokeResponseBody,
  \},
  manager::\{webview::UriSchemeProtocol, AppManager, Asset\},
  plugin::\{Plugin, PluginStore\},
  resources::ResourceTable,
  runtime::\{
    window::\{WebviewEvent as RuntimeWebviewEvent, WindowEvent as RuntimeWindowEvent\},
    ExitRequestedEventAction,
    RunEvent as RuntimeRunEvent,
  \},
  sealed::\{ManagerBase, RuntimeOrDispatch\},
  utils::\{config::Config, Env\},
  webview::PageLoadPayload,
  Context,
  DeviceEventFilter,
  Emitter,
  EventLoopMessage,
  EventName,
  Listener,
  Manager,
  Monitor,
  Runtime,
  Scopes,
  StateManager,
  Theme,
  Webview,
  WebviewWindowBuilder,
  Window,
\};

#[cfg(desktop)]
use crate::menu::Menu;
#[cfg(all(desktop, feature = "tray-icon"))]
use crate::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent, TrayIconId};
use raw_window_handle::HasDisplayHandle;
use serialize_to_javascript::{default_template, DefaultTemplate, Template};
use tauri_macros::default_runtime;
#[cfg(desktop)]
use tauri_runtime::EventLoopProxy;
use tauri_runtime::\{
  dpi::\{PhysicalPosition, PhysicalSize\},
  window::DragDropEvent,
  RuntimeInitArgs,
\};
use tauri_utils::assets::AssetsIter;
use tauri_utils::PackageInfo;

use std::\{
  borrow::Cow,
  collections::HashMap,
  fmt,
  sync::\{
    atomic,
    mpsc::Sender,
    Arc,
    Mutex,
    MutexGuard,
  \},
  thread::ThreadId,
  time::Duration,
\};

use crate::\{event::EventId, runtime::RuntimeHandle, Event, EventTarget\};

#[cfg(target_os = "macos")]
use crate::ActivationPolicy;

pub(crate) mod plugin;

#[cfg(desktop)]
pub(crate) type GlobalMenuEventListener<T> = Box<dyn Fn(&T, crate::menu::MenuEvent) + Send + Sync>;
#[cfg(all(desktop, feature = "tray-icon"))]
pub(crate) type GlobalTrayIconEventListener<T> =
  Box<dyn Fn(&T, crate::tray::TrayIconEvent) + Send + Sync>;
pub(crate) type GlobalWindowEventListener<R> = Box<dyn Fn(&Window<R>, &WindowEvent) + Send + Sync>;
pub(crate) type GlobalWebviewEventListener<R> =
  Box<dyn Fn(&Webview<R>, &WebviewEvent) + Send + Sync>;
/// A closure that is run when the Tauri application is setting up.
pub type SetupHook<R> =
  Box<dyn FnOnce(&mut App<R>) -> std::result::Result<(), Box<dyn std::error::Error>> + Send>;
/// A closure that is run every time a page starts or finishes loading.
pub type OnPageLoad<R> = dyn Fn(&Webview<R>, &PageLoadPayload<'_>) + Send + Sync + 'static;
pub type ChannelInterceptor<R> =
  Box<dyn Fn(&Webview<R>, CallbackFn, usize, &InvokeResponseBody) -> bool + Send + Sync + 'static>;

/// The exit code on [`RunEvent::ExitRequested`] when [`AppHandle#method.restart`] is called.
pub const RESTART_EXIT_CODE: i32 = i32::MAX;

/// Api exposed on the `ExitRequested` event.
#[derive(Debug, Clone)]
pub struct ExitRequestApi {
  tx: Sender<ExitRequestedEventAction>,
  code: Option<i32>,
}

impl ExitRequestApi {
  /// Prevents the app from exiting.
  ///
  /// **Note:** This is ignored when using [`AppHandle#method.restart`].
  pub fn prevent_exit(&self) {
    if self.code != Some(RESTART_EXIT_CODE) {
      self.tx.send(ExitRequestedEventAction::Prevent).unwrap();
    }
  }
}

/// Api exposed on the `CloseRequested` event.
#[derive(Debug, Clone)]
pub struct CloseRequestApi(Sender<bool>);

impl CloseRequestApi {
  /// Prevents the window from being closed.
  pub fn prevent_close(&self) {
    self.0.send(true).unwrap();
  }
}

/// An event from a window.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum WindowEvent {
  /// The size of the window has changed. Contains the client area's new dimensions.
  Resized(PhysicalSize<u32>),
  /// The position of the window has changed. Contains the window's new position.
  Moved(PhysicalPosition<i32>),
  /// The window has been requested to close.
  #[non_exhaustive]
  CloseRequested {
    /// An API modify the behavior of the close requested event.
    api: CloseRequestApi,
  },
  /// The window has been destroyed.
  Destroyed,
  /// The window gained or lost focus.
  ///
  /// The parameter is true if the window has gained focus, and false if it has lost focus.
  Focused(bool),
  /// The window's scale factor has changed.
  ///
  /// The following user actions can cause DPI changes:
  ///
  /// - Changing the display's resolution.
  /// - Changing the display's scale factor (e.g. in Control Panel on Windows).
  /// - Moving the window to a display with a different scale factor.
  #[non_exhaustive]
  ScaleFactorChanged {
    /// The new scale factor.
    scale_factor: f64,
  },
}

```

----------------------------------------

TITLE: Example: Get IPC Response for Ping
DESCRIPTION: Illustrates how to retrieve the response of an IPC message, specifically the 'ping' command, using `get_ipc_response`. This example shows how to deserialize the response and assert its content.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/test/mod

LANGUAGE: rust
CODE:
```
use tauri::test::{mock_builder, mock_context, noop_assets};

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

fn create_app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::App<R> {
    builder
        .invoke_handler(tauri::generate_handler![ping])
        // remove the string argument to use your app's config file
        .build(tauri::generate_context!("test/fixture/src-tauri/tauri.conf.json"))
        .expect("failed to build app")
}

fn main() {
    let app = create_app(mock_builder());
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();

    // run the `ping` command and assert it returns `pong`
    let res = tauri::test::get_ipc_response(
        &webview,
        tauri::webview::InvokeRequest {
            cmd: "ping".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::default(),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        },
    );
    assert!(res.is_ok());
    assert_eq!(res.unwrap().deserialize::<String>().unwrap(), String::from("pong"));
}
```

----------------------------------------

TITLE: Basic Plugin Initialization Example
DESCRIPTION: Demonstrates a simple way to initialize a Tauri plugin using the `Builder` pattern, exporting an `init` function.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/plugin

LANGUAGE: rust
CODE:
```
use tauri::{plugin::{Builder, TauriPlugin}, Runtime};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("example")
    .build()
}
```

----------------------------------------

TITLE: Setup Tauri Event Listeners
DESCRIPTION: Configures event listeners for different Tauri components (App, Window, Webview, WebviewWindow). It uses a macro to simplify the setup process for both specific and any event listeners. This function returns a struct containing the components and channels for event communication.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/manager/mod

LANGUAGE: rust
CODE:
```
fn setup_events(setup_any: bool) -> EventSetup {
    let app = mock_app();

    let window = WindowBuilder::new(&app, "main-window").build().unwrap();

    let webview = window
      .add_child(
        WebviewBuilder::new("main-webview", Default::default()),
        crate::LogicalPosition::new(0, 0),
        window.inner_size().unwrap(),
      )
      .unwrap();

    let webview_window = WebviewWindowBuilder::new(&app, "main-webview-window", Default::default())
      .build()
      .unwrap();

    let (tx, rx) = channel();

    macro_rules! setup_listener {
      ($type:ident, $id:ident, $any_id:ident) => {
        let tx_ = tx.clone();
        $type.listen(TEST_EVENT_NAME, move |evt| {
          tx_.
            send(($id, serde_json::from_str::<String>(evt.payload()).unwrap()))
            .unwrap();
        });

        if setup_any {
          let tx_ = tx.clone();
          $type.listen_any(TEST_EVENT_NAME, move |evt| {
            tx_.
              send((
                $any_id,
                serde_json::from_str::<String>(evt.payload()).unwrap(),
              ))
              .unwrap();
          });
        }
      };
    }

    setup_listener!(app, APP_LISTEN_ID, APP_LISTEN_ANY_ID);
    setup_listener!(window, WINDOW_LISTEN_ID, WINDOW_LISTEN_ANY_ID);
    setup_listener!(webview, WEBVIEW_LISTEN_ID, WEBVIEW_LISTEN_ANY_ID);
    setup_listener!(
      webview_window,
      WEBVIEW_WINDOW_LISTEN_ID,
      WEBVIEW_WINDOW_LISTEN_ANY_ID
    );

    EventSetup {
      app,
      window,
      webview,
      webview_window,
      tx,
      rx,
    }
  }
```

----------------------------------------

TITLE: Basic Plugin Initialization Example (Rust)
DESCRIPTION: Demonstrates a conventional way to initialize a Tauri plugin using the `Builder` pattern. It shows a simple `init` function that creates a plugin with a given name.

SOURCE: https://docs.rs/tauri/latest/src/tauri/plugin

LANGUAGE: rust
CODE:
```
use tauri::{plugin::{Builder, TauriPlugin}, Runtime};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("example")
    .build()
}
```

----------------------------------------

TITLE: Rust MockRuntime Initialization
DESCRIPTION: Provides a mock implementation of the Tauri Runtime for testing purposes. Includes initialization logic and context setup.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/test/mock_runtime

LANGUAGE: rust
CODE:
```
pub struct MockRuntime {
  is_running: Arc<AtomicBool>,
  pub context: RuntimeContext,
  run_rx: Receiver<Message>,
}

impl MockRuntime {
  fn init() -> Self {
    let is_running = Arc::new(AtomicBool::new(false));
    let (tx, rx) = sync_channel(256);
    let context = RuntimeContext {
      is_running: is_running.clone(),
      windows: Default::default(),
      shortcuts: Default::default(),
      run_tx: tx,
      next_window_id: Default::default(),
      next_webview_id: Default::default(),
      next_window_event_id: Default::default(),
      next_webview_event_id: Default::default(),
    };
    Self {
      is_running,
      context,
      run_rx: rx,
    }
  }
}
```

----------------------------------------

TITLE: Setup Tauri Event Listeners
DESCRIPTION: Configures event listeners for different Tauri components (App, Window, Webview, WebviewWindow). It uses a macro to simplify the setup process for both specific and any event listeners. This function returns a struct containing the components and channels for event communication.

SOURCE: https://docs.rs/tauri/latest/src/tauri/manager/mod

LANGUAGE: rust
CODE:
```
fn setup_events(setup_any: bool) -> EventSetup {
    let app = mock_app();

    let window = WindowBuilder::new(&app, "main-window").build().unwrap();

    let webview = window
      .add_child(
        WebviewBuilder::new("main-webview", Default::default()),
        crate::LogicalPosition::new(0, 0),
        window.inner_size().unwrap(),
      )
      .unwrap();

    let webview_window = WebviewWindowBuilder::new(&app, "main-webview-window", Default::default())
      .build()
      .unwrap();

    let (tx, rx) = channel();

    macro_rules! setup_listener {
      ($type:ident, $id:ident, $any_id:ident) => {
        let tx_ = tx.clone();
        $type.listen(TEST_EVENT_NAME, move |evt| {
          tx_.
            send(($id, serde_json::from_str::<String>(evt.payload()).unwrap()))
            .unwrap();
        });

        if setup_any {
          let tx_ = tx.clone();
          $type.listen_any(TEST_EVENT_NAME, move |evt| {
            tx_.
              send((
                $any_id,
                serde_json::from_str::<String>(evt.payload()).unwrap(),
              ))
              .unwrap();
          });
        }
      };
    }

    setup_listener!(app, APP_LISTEN_ID, APP_LISTEN_ANY_ID);
    setup_listener!(window, WINDOW_LISTEN_ID, WINDOW_LISTEN_ANY_ID);
    setup_listener!(webview, WEBVIEW_LISTEN_ID, WEBVIEW_LISTEN_ANY_ID);
    setup_listener!(
      webview_window,
      WEBVIEW_WINDOW_LISTEN_ID,
      WEBVIEW_WINDOW_LISTEN_ANY_ID
    );

    EventSetup {
      app,
      window,
      webview,
      webview_window,
      tx,
      rx,
    }
  }
```

----------------------------------------

TITLE: Create Webview in Setup Hook - Tauri
DESCRIPTION: Demonstrates how to create a webview within the Tauri application's setup hook. It initializes a WebviewBuilder with a label and an app-relative URL, then adds it as a child to a window.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/webview/mod

LANGUAGE: rust
CODE:
```
tauri::Builder::default()
  .setup(|app| {
    let window = tauri::window::WindowBuilder::new(app, "label").build()?;
    let webview_builder = tauri::webview::WebviewBuilder::new("label", tauri::WebviewUrl::App("index.html".into()));
    let webview = window.add_child(webview_builder, tauri::LogicalPosition::new(0, 0), window.inner_size().unwrap());
    Ok(())
  });
```

----------------------------------------

TITLE: Configure and Build Menu
DESCRIPTION: Configures the application menu based on user-defined settings and initializes the application menu, including platform-specific macOS menu setup.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/app

LANGUAGE: rust
CODE:
```
if let Some(menu) = self.menu {
  let menu = menu(&app.handle)?;
  app
    .manager
    .menu
    .menus_stash_lock()
    .insert(menu.id().clone(), menu.clone());

  #[cfg(target_os = "macos")]
  init_app_menu(&menu)?;

  app.manager.menu.menu_lock().replace(menu);
}
```

----------------------------------------

TITLE: Example: Initialize Script in Tauri App
DESCRIPTION: Demonstrates how to set an initialization script for all frames in a Tauri application. The script checks the window origin and logs a message, also setting a custom property on the window object.

SOURCE: https://docs.rs/tauri/latest/src/tauri/webview/mod

LANGUAGE: rust
CODE:
```
use tauri::{WindowBuilder, Runtime};

const INIT_SCRIPT: &str = r#"#
  if (window.location.origin === 'https://tauri.app') {
    console.log("hello world from js init script");

    window.__MY_CUSTOM_PROPERTY__ = { foo: 'bar' };
  }
"#;

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      let window = tauri::window::WindowBuilder::new(app, "label").build()?;
      let webview_builder = tauri::webview::WebviewBuilder::new("label", tauri::WebviewUrl::App("index.html".into()))
        .initialization_script_for_all_frames(INIT_SCRIPT);
      let webview = window.add_child(webview_builder, tauri::LogicalPosition::new(0, 0), window.inner_size().unwrap())?; 
      Ok(())
    });
}
```

----------------------------------------

TITLE: Add Tauri Application Plugin
DESCRIPTION: Adds a Tauri application plugin to the builder. Plugins extend the functionality of Tauri applications. This example demonstrates how to define and add a plugin with commands, setup logic, and event handlers.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/app

LANGUAGE: rust
CODE:
```
mod plugin {
  use tauri::{plugin::{Builder as PluginBuilder, TauriPlugin}, RunEvent, Runtime};

  #[tauri::command]
  async fn do_something<R: Runtime>(app: tauri::AppHandle<R>, window: tauri::Window<R>) -> Result<(), String> {
    println!("command called");
    Ok(())
  }
  pub fn init<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("window")
      .setup(|app, api| {
        Ok(())
      })
      .on_event(|app, event| {
        match event {
          RunEvent::Ready => {
            println!("app is ready");
          }
          RunEvent::WindowEvent { label, event, .. } => {
            println!("window {} received an event: {:?}", label, event);
          }
          _ => (),
        }
      })
      .invoke_handler(tauri::generate_handler![do_something])
      .build()
  }
}

tauri::Builder::default()
  .plugin(plugin::init());

```

----------------------------------------

TITLE: Run Tauri Application
DESCRIPTION: Executes the Tauri application's event loop continuously. This function does not return; the process exits directly. It handles setup and event callbacks, panicking if the setup fails. Includes an example of preventing application exit.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/app

LANGUAGE: rust
CODE:
```
pub fn run<F: FnMut(&AppHandle<R>, RunEvent) + 'static>(mut self, callback: F) {
    self.handle.event_loop.lock().unwrap().main_thread_id = std::thread::current().id();

    self
      .runtime
      .take()
      .unwrap()
      .run(self.make_run_event_loop_callback(callback));
}
```

----------------------------------------

TITLE: Tauri Application Initialization
DESCRIPTION: Initializes the main Tauri application struct, including the runtime, setup function, plugin manager, and application handle. It also manages core plugins and environment settings.

SOURCE: https://docs.rs/tauri/latest/src/tauri/app

LANGUAGE: rust
CODE:
```
let mut app = App {
  runtime: Some(runtime),
  setup: Some(self.setup),
  manager: manager.clone(),
  handle: AppHandle {
    runtime_handle,
    manager,
    event_loop: Arc::new(Mutex::new(EventLoop {
      main_thread_id: std::thread::current().id(),
    })),
  },
  ran_setup: false,
};
```

LANGUAGE: rust
CODE:
```
app.register_core_plugins()?;
let env = Env::default();
app.manage(env);
```

LANGUAGE: rust
CODE:
```
app.manage(Scopes {
  #[cfg(feature = "protocol-asset")]
  asset_protocol: crate::scope::fs::Scope::new(
    &app,
    &app.config().app.security.asset_protocol.scope,
  )?,
});
```

LANGUAGE: rust
CODE:
```
app.manage(ChannelDataIpcQueue::default());
app.handle.plugin(crate::ipc::channel::plugin())?;
```

----------------------------------------

TITLE: Tauri PluginStore: Initialize Plugins
DESCRIPTION: Contains methods for initializing individual plugins or all plugins within the store. Initialization requires the application handle and plugin configuration, returning a Result indicating success or failure.

SOURCE: https://docs.rs/tauri/latest/src/tauri/plugin

LANGUAGE: rust
CODE:
```
pub(crate) fn initialize(
    &self,
    plugin: &mut Box<dyn Plugin<R>>,
    app: &AppHandle<R>,
    config: &PluginConfig,
  ) -> crate::Result<()> {
    initialize(plugin, app, config)
  }

  pub(crate) fn initialize_all(
    &mut self,
    app: &AppHandle<R>,
    config: &PluginConfig,
  ) -> crate::Result<()> {
    self
      .store
      .iter_mut()
      .try_for_each(|plugin| initialize(plugin, app, config))
  }
```

----------------------------------------

TITLE: Rust MockRuntime Initialization
DESCRIPTION: Provides a mock implementation of the Tauri Runtime for testing purposes. Includes initialization logic and context setup.

SOURCE: https://docs.rs/tauri/latest/src/tauri/test/mock_runtime

LANGUAGE: rust
CODE:
```
pub struct MockRuntime {
  is_running: Arc<AtomicBool>,
  pub context: RuntimeContext,
  run_rx: Receiver<Message>,
}

impl MockRuntime {
  fn init() -> Self {
    let is_running = Arc::new(AtomicBool::new(false));
    let (tx, rx) = sync_channel(256);
    let context = RuntimeContext {
      is_running: is_running.clone(),
      windows: Default::default(),
      shortcuts: Default::default(),
      run_tx: tx,
      next_window_id: Default::default(),
      next_webview_id: Default::default(),
      next_window_event_id: Default::default(),
      next_webview_event_id: Default::default(),
    };
    Self {
      is_running,
      context,
      run_rx: rx,
    }
  }
}
```

----------------------------------------

TITLE: Tauri Application Initialization
DESCRIPTION: Initializes the main Tauri application struct, including the runtime, setup function, plugin manager, and application handle. It also manages core plugins and environment settings.

SOURCE: https://docs.rs/tauri/2.8.5/src/tauri/app

LANGUAGE: rust
CODE:
```
let mut app = App {
  runtime: Some(runtime),
  setup: Some(self.setup),
  manager: manager.clone(),
  handle: AppHandle {
    runtime_handle,
    manager,
    event_loop: Arc::new(Mutex::new(EventLoop {
      main_thread_id: std::thread::current().id(),
    })),
  },
  ran_setup: false,
};
```

LANGUAGE: rust
CODE:
```
app.register_core_plugins()?;
let env = Env::default();
app.manage(env);
```

LANGUAGE: rust
CODE:
```
app.manage(Scopes {
  #[cfg(feature = "protocol-asset")]
  asset_protocol: crate::scope::fs::Scope::new(
    &app,
    &app.config().app.security.asset_protocol.scope,
  )?,
});
```

LANGUAGE: rust
CODE:
```
app.manage(ChannelDataIpcQueue::default());
app.handle.plugin(crate::ipc::channel::plugin())?;
```