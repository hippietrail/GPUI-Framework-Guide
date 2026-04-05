# GPUI Framework Guide

A comprehensive guide to building applications with GPUI, Zed's GPU-accelerated UI framework.

## Architecture Overview

GPUI is an immediate-mode-inspired, retained-state UI framework. The element tree is rebuilt every frame, but entity state persists across frames. Layout uses Taffy (a Rust flexbox/grid engine). Rendering goes to a flat scene graph of GPU primitives (quads, shadows, sprites, paths) batched and drawn via wgpu/Metal/DirectX.

### Core Concepts

- **App**: The singleton application state. All entities, windows, and globals live here behind a `RefCell`.
- **Entity\<T\>**: A reference-counted handle to state of type `T`. The ECS primitive.
- **Window**: Holds the platform window, layout engine, frame state, focus, and dispatch tree.
- **Element**: An ephemeral rendering unit. Created, laid out, painted, then dropped every frame.
- **View**: An `Entity<T>` where `T: Render`. The bridge between persistent state and ephemeral elements.

### The Frame Lifecycle

```
State change (entity.update → cx.notify())
  → Effect queue (Notify { entity_id })
  → flush_effects() marks window dirty via entity→window tracking
  → Platform vsync fires on_request_frame
  → window.draw(cx):
      1. Prepaint phase: render() → request_layout() → prepaint() for each element
      2. Paint phase: paint() emits primitives to the scene
      3. Frame swap (double-buffered)
  → window.present() sends scene to GPU
```

## Entities and State Management

### Creating Entities

```rust
// In a window context:
let counter = cx.new(|cx| Counter { count: 0 });

// Reading:
let value = counter.read(cx).count;

// Updating (provides Context<Counter>):
counter.update(cx, |this, cx| {
    this.count += 1;
    cx.notify(); // marks window dirty, triggers redraw
});
```

### Entity Rules

- `entity.update()` temporarily removes the entity from the map. **You cannot recursively update the same entity** -- it panics with "double lease."
- Use `entity.downgrade()` → `WeakEntity<T>` for non-owning references. Prevents reference cycles and memory leaks.
- Inside closures passed to `update`, always use the inner `cx`, not the outer one.

### Observing Changes

```rust
// Watch another entity for changes:
cx.observe(&other_entity, |this, other, cx| {
    // Called when other calls cx.notify()
});

// Watch a global:
cx.observe_global::<MyGlobal>(|this, cx| { ... });

// Subscribe to typed events:
cx.subscribe(&other_entity, |this, other, event: &MyEvent, cx| { ... });
```

### Emitting Events

```rust
impl EventEmitter<ClickEvent> for MyButton {}

// In an update:
cx.emit(ClickEvent { position });
```

## Views and Rendering

### The Render Trait

```rust
struct MyView {
    name: String,
}

impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(cx.theme().colors().background)
            .text_color(cx.theme().colors().text)
            .child(format!("Hello, {}!", self.name))
            .child(
                div()
                    .border_1()
                    .border_color(gpui::red())
                    .rounded_md()
                    .p_2()
                    .child("A bordered box")
            )
    }
}
```

### RenderOnce (Stateless Components)

```rust
#[derive(IntoElement)]
struct Badge {
    label: SharedString,
    color: Hsla,
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_full()
            .bg(self.color)
            .text_xs()
            .text_color(gpui::white())
            .child(self.label)
    }
}

// Usage: .child(Badge { label: "New".into(), color: gpui::blue() })
```

### Opening Windows

```rust
fn main() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(|_| MyView { name: "World".into() }),
        )
        .unwrap();
    });
}
```

## Elements

### div() -- The Universal Container

The core building block. Implements `Styled`, `InteractiveElement`, `ParentElement`.

```rust
div()
    .id("my-div")              // required for stateful interactions
    .flex()                     // display: flex
    .flex_col()                 // flex-direction: column
    .gap_2()                    // gap: 0.5rem
    .p_4()                      // padding: 1rem
    .w_full()                   // width: 100%
    .h_48()                     // height: 12rem
    .bg(gpui::rgb(0x1e1e2e))   // background color
    .border_1()                 // border-width: 1px
    .border_color(gpui::rgb(0x444444))
    .rounded_md()               // border-radius: medium
    .shadow_lg()                // box shadow
    .overflow_hidden()          // clip overflow
    .cursor_pointer()           // cursor style
    .text_sm()                  // font-size: small
    .text_color(gpui::white())  // text color
    .child("text content")      // add child (strings implement IntoElement)
    .child(other_element)       // add element child
    .children(vec_of_elements)  // add multiple children
    .when(condition, |this| this.bg(gpui::red()))  // conditional styling
    .when_some(option, |this, value| this.child(value))  // conditional child
```

### Text

Plain strings implement `IntoElement`:

```rust
div().child("Hello world")         // &str
div().child(format!("Count: {}", n)) // String
div().child(SharedString::from("shared")) // SharedString (avoids cloning)
```

Styled text with highlight ranges:

```rust
StyledText::new("Hello world")
    .with_highlights(&cx.text_style(), vec![
        (0..5, HighlightStyle { color: Some(gpui::red()), ..Default::default() }),
    ])
```

### img() -- Images

```rust
img("/path/to/image.png")         // from file
img("https://example.com/pic.jpg") // from URL (async)
img(SharedUri::from(url))          // explicit URI
    .size_8()                       // 2rem x 2rem
    .rounded_full()                 // circular
    .object_fit(ObjectFit::Cover)   // cover/contain/fill
    .grayscale(true)
    .with_fallback(|| img("fallback.png"))
    .with_loading(|| div().child("Loading..."))
```

### svg() -- SVG Icons

```rust
svg()
    .path("icons/check.svg")       // from bundled assets
    .size_4()                       // 1rem x 1rem
    .text_color(gpui::green())      // SVG fill color = text_color
    .with_transformation(Transformation::rotate(percentage(0.25)))
```

### uniform_list() -- Virtual Scrolling (Same-Height Items)

Only measures one item, uses that height for all. Best for large homogeneous lists.

```rust
uniform_list("items", item_count, cx.processor(|this, range: Range<usize>, _, _| {
    // Only called for visible range
    range.map(|i| this.render_item(i)).collect()
}))
.size_full()
.track_scroll(&self.scroll_handle)
```

`ScrollStrategy`: `Top`, `Center`, `Bottom`, `Nearest`.

### list() -- Virtual Scrolling (Variable-Height Items)

```rust
let state = ListState::new(item_count, ListAlignment::Top, px(1000.), render_fn);
list(state, |ix, window, cx| render_item(ix, window, cx))
```

### canvas() -- Custom Drawing

Two-phase: prepaint returns state, paint draws.

```rust
canvas(
    |bounds, window, cx| {
        // Prepaint: compute layout, register hitboxes
        let hitbox = window.insert_hitbox(bounds, false);
        hitbox
    },
    |bounds, hitbox, window, cx| {
        // Paint: draw primitives
        window.paint_quad(fill(bounds, gpui::red()));
    },
)
.size_full()
```

### deferred() -- Paint on Top

Paints after parent. Layout stays in place, but rendering is deferred.

```rust
deferred(
    div().absolute().child(popover_content)
).with_priority(1)
```

### anchored() -- Overflow-Safe Positioning

For tooltips, popovers. Snaps to window edges.

```rust
anchored()
    .position(point)
    .anchor(Corner::TopLeft)
    .snap_to_window()
    .child(tooltip_content)
```

## Styling System

GPUI uses Tailwind CSS-inspired method names. All return `Self` for chaining.

### Layout (Flexbox)

| Method | CSS Equivalent |
|--------|---------------|
| `.flex()` | `display: flex` |
| `.flex_col()` | `flex-direction: column` |
| `.flex_row()` | `flex-direction: row` |
| `.flex_wrap()` | `flex-wrap: wrap` |
| `.flex_1()` | `flex: 1 1 0%` |
| `.flex_none()` | `flex: none` |
| `.flex_grow()` | `flex-grow: 1` |
| `.flex_shrink_0()` | `flex-shrink: 0` |
| `.items_center()` | `align-items: center` |
| `.justify_center()` | `justify-content: center` |
| `.justify_between()` | `justify-content: space-between` |
| `.gap_2()` | `gap: 0.5rem` |

### Layout (Grid)

```rust
div()
    .grid()
    .grid_cols(3)
    .gap_2()
    .child(div().col_span(2).child("spans 2 columns"))
    .child(div().child("1 column"))
```

### Sizing

| Method | CSS | Notes |
|--------|-----|-------|
| `.w(px(200.))` | `width: 200px` | Explicit pixels |
| `.w_full()` | `width: 100%` | |
| `.h_48()` | `height: 12rem` | Tailwind scale |
| `.size_full()` | `width: 100%; height: 100%` | |
| `.min_w_0()` | `min-width: 0` | Important for flex truncation |
| `.max_h(px(500.))` | `max-height: 500px` | |
| `.aspect_square()` | `aspect-ratio: 1` | |

### Spacing

| Method | CSS |
|--------|-----|
| `.p_4()` | `padding: 1rem` |
| `.px_2()` | `padding-left/right: 0.5rem` |
| `.py_1()` | `padding-top/bottom: 0.25rem` |
| `.m_2()` | `margin: 0.5rem` |
| `.mt_4()` | `margin-top: 1rem` |

### Position

```rust
div()
    .relative()    // position: relative (default)
    .child(
        div()
            .absolute()   // position: absolute
            .top_0()       // top: 0
            .right_0()     // right: 0
            .size_4()
    )
```

### Visual

```rust
div()
    .bg(gpui::rgb(0x1e1e2e))       // background
    .border_1()                     // 1px border
    .border_color(gpui::rgb(0x333))
    .border_dashed()                // dashed border style
    .rounded_md()                   // border-radius
    .shadow_lg()                    // box-shadow
    .opacity(0.5)                   // opacity
    .overflow_hidden()              // overflow: hidden
    .cursor_pointer()               // cursor style
```

### Text Styling

```rust
div()
    .text_sm()                     // font-size: 0.875rem
    .text_color(gpui::white())     // color
    .font_weight(FontWeight::BOLD) // font-weight
    .italic()                      // font-style: italic
    .underline()                   // text-decoration
    .text_ellipsis()               // text-overflow: ellipsis
    .truncate()                    // overflow:hidden + text-overflow:ellipsis + whitespace:nowrap
    .line_clamp(3)                 // max 3 lines
    .font_family("Berkeley Mono")  // font-family
    .font_features(FontFeatures::from_iter([("calt", 0)])) // OpenType features
```

### Conditional Styling

```rust
div()
    .when(is_selected, |this| this.bg(gpui::blue()).text_color(gpui::white()))
    .when_some(icon, |this, icon| this.child(svg().path(icon)))
```

### Debug

```rust
div()
    .debug()        // red outline on this element
    .debug_below()  // red outline on all descendants
```

## Interaction and Events

### Making Elements Interactive

Most interactions require an element ID (`.id()`):

```rust
div()
    .id("my-button")
    .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
        this.handle_click(cx);
    }))
    .hover(|style| style.bg(gpui::rgb(0x333)))
    .active(|style| style.bg(gpui::rgb(0x222)))
    .cursor_pointer()
```

### Mouse Events

```rust
div()
    .id("canvas")
    .on_mouse_down(MouseButton::Left, cx.listener(|this, event, window, cx| { ... }))
    .on_mouse_up(MouseButton::Left, cx.listener(|this, event, window, cx| { ... }))
    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| { ... }))
    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| { ... }))
```

### Focus and Keyboard

```rust
struct MyInput {
    focus_handle: FocusHandle,
}

impl MyInput {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle() }
    }
}

impl Focusable for MyInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MyInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .key_context("MyInput")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| { ... }))
            .on_action(cx.listener(Self::handle_enter))
            .focus(|style| style.border_color(gpui::blue()))
            .child("input content")
    }
}
```

### Actions

Actions are typed commands dispatched via keybindings or code:

```rust
// Define actions:
actions!(my_app, [Save, Undo, Redo]);

// Or with data:
#[derive(Clone, PartialEq, Action)]
struct GoToLine { line: u32 }

// Bind to keys:
KeyBinding::new("cmd-s", Save, Some("Editor"))

// Handle:
div()
    .on_action(cx.listener(|this, _: &Save, window, cx| {
        this.save(cx);
    }))
```

Actions bubble up the focus tree by default and **stop propagation automatically**. Call `cx.propagate()` in the handler to continue bubbling.

### Drag and Drop

```rust
// Source:
div()
    .id("draggable")
    .on_drag(DraggedItem { id: 42 }, |item, window, cx| {
        // Return the drag ghost element
        div().child(format!("Dragging item {}", item.id))
    })

// Target:
div()
    .id("drop-zone")
    .drag_over::<DraggedItem>(|style, _, _| style.bg(gpui::blue().opacity(0.2)))
    .on_drop(cx.listener(|this, item: &DraggedItem, window, cx| {
        this.receive(item.id, cx);
    }))
```

### Tooltips

```rust
div()
    .id("btn")
    .tooltip(|window, cx| Tooltip::text("Click me"))
    .child("Button")
```

## Concurrency

### Foreground Tasks (Main Thread)

```rust
cx.spawn(async move |this, cx| {
    // `this` is WeakEntity<Self>, `cx` is &mut AsyncApp
    let data = fetch_data().await;
    this.update(cx, |this, cx| {
        this.data = data;
        cx.notify();
    })?;
    Ok(())
}).detach_and_log_err(cx);
```

### Background Tasks

```rust
let task = cx.background_spawn(async move {
    expensive_computation()
});

// Await in a foreground task:
cx.spawn(async move |this, cx| {
    let result = task.await;
    this.update(cx, |this, cx| {
        this.result = result;
        cx.notify();
    })?;
    Ok(())
}).detach();
```

### Task Lifecycle

- `task.detach()` -- runs to completion, result ignored
- `task.detach_and_log_err(cx)` -- same but logs errors
- Store in a field -- cancelled when the struct drops
- `Task::ready(value)` -- immediately resolved task

## Animation

```rust
use gpui::{Animation, AnimationExt, Transformation, percentage, bounce, ease_in_out};

svg()
    .path("icons/spinner.svg")
    .with_animation(
        "spin",
        Animation::new(Duration::from_secs(1)).repeat(),
        |element, delta| {
            element.with_transformation(Transformation::rotate(percentage(delta)))
        },
    )
```

Built-in easing: `linear`, `ease_in_out`, `ease_out_quint()`, `bounce(inner)`, `pulsating_between(min, max)`.

## Theming

### Accessing Theme Colors

```rust
impl Render for MyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        div()
            .bg(colors.background)
            .text_color(colors.text)
            .border_color(colors.border)
    }
}
```

### Theme Color Categories

- **Backgrounds**: `background`, `surface_background`, `elevated_surface_background`, `element_background`, `element_hover`, `element_active`, `element_selected`, `ghost_element_*`, `drop_target_background`
- **Text**: `text`, `text_muted`, `text_placeholder`, `text_disabled`, `text_accent`
- **Borders**: `border`, `border_variant`, `border_focused`, `border_selected`, `border_transparent`, `border_disabled`
- **Editor**: `editor_foreground`, `editor_background`, `editor_gutter_background`, `editor_line_number`, `editor_active_line_background`, `editor_highlighted_line_background`
- **Status**: via `cx.theme().status()` -- `error`, `warning`, `info`, `success`, `hint`, `conflict`, `modified`, etc.

### Creating a Custom Theme (JSON)

```json
{
  "$schema": "https://zed.dev/schema/themes/v0.2.0.json",
  "name": "My Theme Family",
  "author": "Me",
  "themes": [
    {
      "name": "My Dark Theme",
      "appearance": "dark",
      "style": {
        "background": "#1a1b26ff",
        "text": "#c0caf5ff",
        "border": "#3b4261ff",
        "editor.background": "#1a1b26ff",
        "editor.foreground": "#c0caf5ff",
        "syntax": {
          "keyword": { "color": "#bb9af7ff", "font_style": "italic" },
          "string": { "color": "#9ece6aff" },
          "function": { "color": "#7aa2f7ff" },
          "type": { "color": "#2ac3deff" },
          "comment": { "color": "#565f89ff", "font_style": "italic" },
          "variable": { "color": "#c0caf5ff" },
          "number": { "color": "#ff9e64ff" },
          "operator": { "color": "#89ddfeff" }
        },
        "players": [
          { "cursor": "#7aa2f7ff", "background": "#7aa2f7ff", "selection": "#7aa2f73d" }
        ]
      }
    }
  ]
}
```

Only specified fields are overridden. All others fall back to the built-in defaults. Colors are 8-char hex (#rrggbbaa).

### Syntax Token Names

The standard tree-sitter capture names that map to theme syntax colors:

`attribute`, `boolean`, `comment`, `comment.doc`, `constant`, `constructor`, `embedded`, `emphasis`, `emphasis.strong`, `enum`, `function`, `function.method`, `function.definition`, `hint`, `keyword`, `label`, `link_text`, `link_uri`, `number`, `operator`, `predictive`, `preproc`, `primary`, `property`, `punctuation`, `punctuation.bracket`, `punctuation.delimiter`, `string`, `string.escape`, `string.regex`, `string.special`, `string.special.symbol`, `tag`, `text.literal`, `title`, `type`, `variable`, `variable.special`, `variant`

Hierarchical matching: `function.method` matches `function.method` first, falls back to `function`.

## Fonts

### Configuration

In settings.json:

```json
{
  "buffer_font_family": "Berkeley Mono",
  "buffer_font_size": 14,
  "buffer_font_weight": 400,
  "buffer_font_features": { "calt": true, "ss01": true },
  "buffer_font_fallbacks": ["Noto Sans Mono"],
  "buffer_line_height": { "custom": 1.5 },
  "ui_font_family": "Inter",
  "ui_font_size": 16,
  "ui_font_features": { "calt": false }
}
```

### Font Sizes

Clamped to 6-100px. `buffer_line_height` can be `"comfortable"` (1.618), `"standard"` (1.3), or `{"custom": N}`.

### In Code

```rust
let settings = ThemeSettings::get_global(cx);
let buffer_font = &settings.buffer_font;
let ui_font = &settings.ui_font;

// Or directly on elements:
div()
    .font_family("Berkeley Mono")
    .text_size(px(14.))
    .font_weight(FontWeight::MEDIUM)
```

### Text System Architecture

Three platform backends:
- **macOS**: CoreText (native)
- **Windows**: DirectWrite (native)
- **Linux/FreeBSD**: cosmic-text (fontdb + harfbuzz-rs + swash)

Font discovery: cosmic-text uses fontconfig on Linux/FreeBSD. Shaping uses HarfBuzz (`Shaping::Advanced`). Rasterization uses swash.

## Tree-Sitter Integration

### How Syntax Highlighting Works

```
Source text
  → tree-sitter parse (WASM engine)
  → SyntaxMap (layered parse trees, supports language injection)
  → SyntaxMapCaptures (iterate query matches)
  → HighlightId (capture name → index)
  → SyntaxTheme lookup (index → HighlightStyle with color/weight/style)
  → TextRun (styled text runs)
  → ShapedLine (shaped glyphs via cosmic-text/CoreText)
  → Scene (GPU sprites)
```

### Display Map Pipeline

The editor transforms text through five layers:

```
text::Buffer (raw CRDT text)
  → InlayMap (inlay hints, ghost text)
  → FoldMap (code folding)
  → TabMap (tab expansion)
  → WrapMap (soft wrapping)
  → BlockMap (block decorations, diagnostics)
  → DisplayMap (final pixel coordinates)
```

Each layer transforms both text coordinates and highlight IDs.

### Adding a Language

1. Create `crates/grammars/src/<lang>/config.toml`:
```toml
name = "MyLang"
grammar = "mylang"
path_suffixes = ["ml"]
line_comments = ["# "]
brackets = [
    { start = "{", end = "}", close = true, newline = true },
    { start = "(", end = ")", close = true, newline = false },
]
```

2. Add `highlights.scm`:
```scheme
(identifier) @variable
(function_declaration name: (identifier) @function.definition)
(string_literal) @string
(number_literal) @number
(comment) @comment
["if" "else" "for" "while" "return"] @keyword
```

3. Optionally add `indents.scm`, `brackets.scm`, `outline.scm`, `injections.scm`.

4. Register the grammar (native or WASM) in the language registry.

## Gotchas

### State and Borrowing

1. **Double-lease panic**: You cannot call `entity.update(cx, ...)` on an entity that's already being updated. The entity is removed from the map during update.
2. **RefCell at the root**: The entire `App` is behind a `RefCell`. `borrow_mut()` panics if already borrowed. This is why async contexts (`AsyncApp`) must go through `handle.update()`.
3. **`cx.notify()` is essential**: Without it, state changes are invisible. The window won't redraw.
4. **Clone before closures**: When capturing entities in async closures, clone them first (or use variable shadowing).

### Rendering

5. **Elements are ephemeral**: The element tree is rebuilt every frame. Don't store state in elements -- use `element_state` or entity fields.
6. **Element state persistence**: Element state persists via `GlobalElementId` (the path of element IDs from root to element). Changing the tree structure can cause state to be associated with the wrong element.
7. **View caching**: Unchanged views replay previous draw operations. If your render depends on external state not tracked by the entity system, it may not update.
8. **`min_w_0()` for flex truncation**: Flex items have an implicit `min-width: auto`. Add `.min_w_0()` to allow text truncation to work.
9. **Stateful interactions need `.id()`**: Click, drag, hover (style), tooltip, scroll, active styles -- all require `.id("name")` on the element. Without it, you get a debug assertion.

### Actions

10. **Actions stop propagation by default**: In the bubble phase, matched actions stop propagating. Call `cx.propagate()` to continue.
11. **Action handlers use `cx.listener()`**: Always wrap with `cx.listener(|this, action, window, cx| ...)` to get the entity context.

### Performance

12. **`RUST_LIB_BACKTRACE=0` on FreeBSD**: The `anyhow` crate captures backtraces on every error. GPUI creates anyhow errors frequently during normal rendering. On FreeBSD, backtrace capture is catastrophically slow due to `dl_iterate_phdr` taking a global write lock. Set `RUST_LIB_BACKTRACE=0`.
13. **Virtual lists for large data**: Use `uniform_list` (same-height) or `list` (variable-height). Never render thousands of elements directly.
14. **`SharedString` for static text**: Use `SharedString` instead of `String` for text that doesn't change per-frame. It's either `&'static str` or `Arc<str>`.

### Platform

15. **Font fallback chains differ per platform**: macOS uses CoreText (system fallback), Linux/FreeBSD use cosmic-text with explicit fallback lists.
16. **First draw happens synchronously**: `open_window` calls `window.draw()` before returning. Your root view's `render()` must succeed on the first call.

## UI Component Library

The `ui` crate provides higher-level components:

### Buttons

```rust
Button::new("save", "Save File")
    .style(ButtonStyle::Filled)
    .on_click(cx.listener(|this, _, window, cx| this.save(cx)))
    .tooltip(|_, cx| Tooltip::text("Save"))

IconButton::new("close", IconName::X)
    .on_click(cx.listener(|this, _, _, cx| this.close(cx)))
```

`ButtonStyle`: `Filled`, `Tinted(color)`, `Outlined`, `Subtle` (default), `Transparent`.

### Labels

```rust
Label::new("Status: Ready")
    .color(Color::Muted)
    .size(LabelSize::Small)

HighlightedLabel::new("query match", highlight_ranges)
```

### Checkboxes and Toggles

```rust
checkbox("show-hidden", ToggleState::Selected)
    .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx)))

switch("auto-save", ToggleState::Unselected)
    .on_click(cx.listener(|this, _, _, cx| this.toggle_auto_save(cx)))
```

### Other Components

- `Avatar`, `Facepile` -- user avatars
- `ContextMenu`, `PopoverMenu`, `DropdownMenu`, `RightClickMenu` -- menus
- `Modal` -- modal dialogs
- `Tab`, `TabBar` -- tabs
- `Tooltip` -- tooltips
- `Scrollbar` -- scrollbar overlays
- `Progress` -- progress bars
- `Divider` -- visual separators
- `Indicator` -- status dots
- `KeyBinding` -- keyboard shortcut display
- `Navigable` -- keyboard-navigable container
- `Disclosure` -- collapsible sections

## Building a Text Editor

### Option 1: Use the Editor Crate (Recommended)

The `editor` crate provides a complete, production-grade code editor as a reusable GPUI component. For any app that needs a real code editor (not just a text field), use it directly:

```rust
// Initialize the theme system first (required for editor colors):
theme::init(cx);
theme_settings::init(cx);

// Create a buffer with a language:
let buffer = cx.new(|cx| {
    language::Buffer::local("fn main() {\n    println!(\"hello\");\n}\n", cx)
});

// Optionally set the language for syntax highlighting:
buffer.update(cx, |buf, cx| {
    buf.set_language(Some(rust_language.clone()), cx);
});

// Wrap in MultiBuffer (the editor always works through MultiBuffer):
let multi_buffer = cx.new(|cx| MultiBuffer::singleton(buffer, cx));

// Create the editor:
let editor = cx.new(|cx| {
    Editor::for_multibuffer(multi_buffer, None, true, window, cx)
});

// Use it as a child in any view:
impl Render for MyApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.editor.clone())
    }
}
```

The editor automatically uses the active Zed theme (syntax colors, gutter, backgrounds, cursors, selections -- everything). Change the theme globally and all editors update.

The `Editor` is language-agnostic. You don't build "a Rust editor" or "a Python editor." You build **the editor** and plug in a `Language` (grammar + highlight queries) and optionally an LSP adapter.

#### What You Get for Free (~30k Lines)

The editor crate provides all of this out of the box:

- Multi-cursor editing (Cmd+D, Alt+Click)
- Full undo/redo with transaction grouping (edits within 300ms auto-group)
- Mouse and keyboard selection (word, line, block)
- Copy/paste/cut with multi-cursor support
- Auto-indent, auto-close brackets/quotes
- Find/replace with regex support
- IME support (Chinese/Japanese/Korean input methods)
- Scroll anchoring (scroll position stays on the same line after edits above)
- Code folding
- Soft wrapping at viewport width
- Line numbers with relative line number support
- Git gutter (added/modified/deleted indicators)
- Git blame (inline, per-line)
- Diagnostic underlines (error/warning squiggles) and inline messages
- Completion/autocomplete popup
- Hover tooltips (type info, docs)
- Go-to-definition on Cmd+Click
- Inlay hints (type annotations, parameter names)
- Indent guides (vertical lines)
- Cursor shapes: Bar, Block, Hollow, Underline (Vim modes)
- Minimap
- Scrollbars with document overview
- Sticky headers (scope context at top of viewport)
- Block decorations (custom elements between lines)

#### Editor Modes

```rust
EditorMode::SingleLine  // Single-line input (search box, rename field)
EditorMode::AutoHeight  // Grows with content (chat input, commit message)
EditorMode::Full        // Full editor, fills container (main code editor)
```

`Editor::single_line(window, cx)` creates a minimal single-line input backed by an empty buffer.

### Architecture of the Editor

#### Text Storage: The CRDT Buffer

The text lives in `text::Buffer` -- a CRDT (Conflict-free Replicated Data Type) designed for real-time collaboration:

```
text::Buffer
  ├── BufferSnapshot
  │     ├── visible_text: Rope          // The actual text
  │     ├── deleted_text: Rope          // Tombstones (for CRDT)
  │     ├── fragments: SumTree<Fragment> // The CRDT fragment tree
  │     └── version: clock::Global      // Vector clock
  ├── History
  │     ├── undo_stack: Vec<HistoryEntry>
  │     ├── redo_stack: Vec<HistoryEntry>
  │     └── group_interval: Duration    // Auto-groups edits within this window
  └── lamport_clock: clock::Lamport
```

Each edit creates `Fragment` objects with Lamport timestamps. Deleted text isn't removed -- fragments are tombstoned (`visible = false`). Undo/redo toggles undo counts on fragments rather than replaying operations (critical for CRDT correctness).

Edits go through transactions:
```rust
buffer.edit(
    edits.iter().map(|e| (e.range.clone(), e.new_text.clone())),
    Some(AutoindentMode::Block { ... }),
    cx,
);
```

#### Language Buffer: Tree-Sitter Integration

`language::Buffer` wraps the text buffer and adds parsing:

```
language::Buffer
  ├── text: text::Buffer               // The CRDT text
  ├── language: Option<Arc<Language>>   // Grammar + queries
  ├── syntax_map: Mutex<SyntaxMap>      // Tree-sitter parse trees
  ├── reparse: Option<Task<()>>         // Background reparse task
  └── diagnostics: TreeMap<...>         // LSP diagnostics
```

When any edit occurs, `did_edit()` triggers `reparse()`:
1. Calls `syntax_map.interpolate(&text)` -- cheaply adjusts tree positions (tree-sitter supports incremental parsing)
2. Attempts a **synchronous reparse** with a budget (~300ms). Small edits complete inline with no flicker
3. If sync parse takes too long, spawns a **background task** for the full reparse
4. On completion, emits `BufferEvent::Reparsed` so the editor knows highlighting changed
5. If the buffer changed while parsing, automatically reparses again

The `SyntaxMap` holds **layered** parse trees -- one per injected language. A JavaScript file with template literals gets a JS tree and embedded HTML/CSS trees. An HTML file gets HTML + JS + CSS trees. Query results from all layers are merged and sorted by position.

#### The Display Pipeline: Buffer to Screen

This is the core architecture -- five transformation layers between raw text and pixels:

```
text::Buffer (raw CRDT text, byte offsets)
     │
     ▼
  InlayMap (inserts virtual text: inlay hints, ghost text, edit predictions)
     │  Coordinate: InlayPoint
     ▼
  FoldMap (collapses folded code regions to a placeholder "⋯")
     │  Coordinate: FoldPoint
     ▼
  TabMap (expands hard tabs \t to spaces based on tab_size setting)
     │  Coordinate: TabPoint
     ▼
  WrapMap (soft wraps lines to viewport width -- runs async, can be expensive)
     │  Coordinate: WrapPoint
     ▼
  BlockMap (inserts block decorations between lines: diagnostics, file headers)
     │  Coordinate: BlockPoint → DisplayPoint
     ▼
  DisplaySnapshot (final pixel coordinates, ready for rendering)
```

Each layer follows the same pattern:
- A **Transform** type representing regions it manages
- A **Snapshot** capturing immutable state at a point in time
- A **sync** function that takes edits from the layer below and produces edits for the layer above
- Its own **coordinate newtype** wrapping a Point

The sync cascade runs every time the editor needs a fresh snapshot:
```rust
fn sync(&mut self, cx: &mut App) {
    let buffer_snapshot = self.buffer.read(cx).snapshot(cx);
    let edits = self.buffer_subscription.consume();
    let (snapshot, edits) = self.inlay_map.sync(buffer_snapshot, edits);
    let (snapshot, edits) = self.fold_map.read(snapshot, edits);
    let (snapshot, edits) = self.tab_map.sync(snapshot, edits, tab_size);
    let (snapshot, edits) = self.wrap_map.sync(snapshot, edits, cx);
    let block_snapshot = self.block_map.read(snapshot, edits);
}
```

Coordinate conversion drills through all layers:
```rust
fn point_to_display_point(buffer_point) -> DisplayPoint {
    let inlay = inlay_snapshot.to_inlay_point(buffer_point);
    let fold = fold_snapshot.to_fold_point(inlay, bias);
    let tab = tab_snapshot.fold_point_to_tab_point(fold);
    let wrap = wrap_snapshot.tab_point_to_wrap_point(tab);
    let block = block_snapshot.to_block_point(wrap);
    DisplayPoint(block)
}
```

Each layer also passes through **highlight IDs** from tree-sitter, so syntax colors survive all transformations unchanged.

#### Only Visible Lines Are Rendered

The editor calculates the visible row range during `prepaint`:

```rust
let scroll_position = snapshot.scroll_position(); // fractional row/col
let start_row = (scroll_position.y).floor();
let end_row = (scroll_position.y + viewport_height / line_height).ceil();
```

Only rows in `start_row..end_row` get:
- `highlighted_chunks()` iterated (tree-sitter highlights merged with search/selection highlights)
- Text shaped via `shape_line()` (cosmic-text/CoreText/DirectWrite)
- Laid out into `LineWithInvisibles` structs
- Cursors, selections, diagnostics computed

Everything outside the viewport is skipped entirely. This is how the editor handles million-line files with constant memory and frame time.

#### How a Line Goes From Buffer to Pixels

```
1. highlighted_chunks(visible_row_range)
   → iterates through BlockMap → WrapMap → TabMap → FoldMap → InlayMap → Buffer
   → yields chunks: { text: &str, syntax_highlight_id, diagnostic_severity }

2. HighlightId → SyntaxTheme lookup
   → HighlightStyle { color, font_weight, font_style, background_color }

3. Merge with overlay highlights (search matches, selections, diagnostics)
   → final HighlightStyle per chunk

4. Build TextRun array: [{ font, color, background, underline, len }]

5. window.text_system().shape_line(text, font_size, &runs, wrapping)
   → ShapedLine (positioned glyphs with kerning, ligatures)

6. ShapedLine.paint(origin, line_height, window, cx)
   → emits MonochromeSprite / SubpixelSprite primitives to the scene

7. Scene → GPU render pass → pixels on screen
```

#### The EditorElement: GPUI Element Implementation

The `Render` impl on `Editor` is minimal:
```rust
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        EditorElement::new(&cx.entity(), self.create_style(cx))
    }
}
```

All real rendering happens in `EditorElement`, which implements the three GPUI `Element` phases:

**Phase 1: `request_layout`** -- Determines editor size based on mode:
- `SingleLine`: height = one line, width = 100%
- `AutoHeight`: measured callback calculates height from content line count
- `Full`: fills container (100% × 100%)

**Phase 2: `prepaint`** -- The heavy phase. Produces an `EditorLayout`:
1. Resolves font metrics: font_id, font_size, line_height, em_width, em_advance
2. Calculates gutter width (line numbers, git gutter, breakpoints, fold controls)
3. Calculates text area width = bounds.width - gutter_width
4. Sets wrap width on the display map, gets a fresh `DisplaySnapshot`
5. Calculates visible row range from scroll position
6. **Shapes visible lines**: calls `highlighted_chunks()` → `LineWithInvisibles::from_chunks()` → `shape_line()`
7. Computes cursor positions via `ShapedLine::x_for_index(byte_offset)` → pixel X
8. Builds selection rectangles, diagnostic ranges, search highlights
9. Lays out block decorations (diagnostic messages, file headers)
10. Packs everything into `EditorLayout` and `PositionMap`

**Phase 3: `paint`** -- Draws in this order:
1. `window.handle_input(...)` -- connects IME (must happen in paint)
2. Registers all action handlers and key listeners
3. Paints background
4. Paints indent guides (vertical lines)
5. Paints git blame in gutter
6. Paints line numbers
7. Paints text area:
   - Selection rectangles (colored quads behind text)
   - Shaped text lines (`ShapedLine.paint()`)
   - Cursors (Bar: 2px wide; Block: character width; Hollow: outline; Underline: 2px tall)
   - Inline diagnostics, document colors
8. Paints block decorations (diagnostic messages, custom elements)
9. Paints sticky headers (scope context at viewport top)
10. Paints minimap
11. Paints scrollbars

#### Key Layout Types

```rust
// Everything needed to paint one frame:
struct EditorLayout {
    position_map: Rc<PositionMap>,      // Line layouts + coordinate mapping
    visible_display_row_range: Range<DisplayRow>,
    line_numbers: HashMap<MultiBufferRow, LineNumberLayout>,
    visible_cursors: Vec<CursorLayout>,
    selections: Vec<(PlayerColor, Vec<SelectionLayout>)>,
    highlighted_ranges: Vec<(Range<DisplayPoint>, Hsla)>,
    blocks: Vec<BlockLayout>,
    // ... 30+ more fields
}

// Maps between pixel positions and text positions:
struct PositionMap {
    size: Size<Pixels>,
    line_height: Pixels,
    scroll_pixel_position: Point<ScrollPixelOffset>,
    em_width: Pixels,
    line_layouts: Vec<LineWithInvisibles>,  // The shaped lines
    snapshot: EditorSnapshot,
    text_hitbox: Hitbox,
}

// One shaped line with invisible character markers:
struct LineWithInvisibles {
    fragments: SmallVec<[LineFragment; 1]>,
    width: Pixels,
}

enum LineFragment {
    Text(ShapedLine),           // Shaped text from the text system
    Element { element, size },  // Inline element (widget, image)
}

// Cursor rendering data:
struct CursorLayout {
    origin: Point<Pixels>,
    block_width: Pixels,
    line_height: Pixels,
    color: Hsla,
    shape: CursorShape,              // Bar, Block, Hollow, Underline
    block_text: Option<ShapedLine>,  // Character under block cursor
    cursor_name: Option<AnyElement>, // Collaborator name label
}
```

#### Scroll Management

The `ScrollManager` maintains scroll state that survives edits:

- `scroll_position: Point<ScrollOffset>` -- fractional row/column offset (e.g., row 42.5 = halfway through row 42)
- `scroll_anchor: ScrollAnchor` -- a buffer `Anchor` that survives edits. If someone inserts 10 lines above your viewport, the anchor adjusts so you stay looking at the same code
- `visible_line_count: Option<f64>` -- set during prepaint from `viewport_height / line_height`

Autoscroll keeps the cursor visible:
```rust
editor.scroll_manager.autoscroll_requested = Some(Autoscroll::contain());
// or: Autoscroll::center(), Autoscroll::fit(), Autoscroll::newest()
```

### IME (Input Method Editor) Integration

The `EntityInputHandler` trait is the contract between your editor and the platform's input method system. All offsets are **UTF-16** because that's what macOS/Windows IME APIs use.

```rust
pub trait EntityInputHandler: 'static + Sized {
    /// Platform asks: what text is at this range?
    fn text_for_range(&mut self, range: Range<usize>, cx: &mut App) -> Option<String>;

    /// Platform asks: what is currently selected?
    fn selected_text_range(&mut self, cx: &mut App) -> Option<UTF16Selection>;

    /// Where is the IME composition text (underlined in-progress text)?
    fn marked_text_range(&self, cx: &App) -> Option<Range<usize>>;

    /// IME composition ended -- remove the composition markers
    fn unmark_text(&mut self, cx: &mut App);

    /// Finalized text from keyboard or IME
    fn replace_text_in_range(
        &mut self, range: Option<Range<usize>>, text: &str, cx: &mut App,
    );

    /// In-progress IME composition (e.g., typing Chinese/Japanese/Korean)
    fn replace_and_mark_text_in_range(
        &mut self, range: Option<Range<usize>>, new_text: &str,
        new_selected_range: Option<Range<usize>>, cx: &mut App,
    );

    /// Where to position the IME candidate popup (pixel bounds)
    fn bounds_for_range(
        &mut self, range: Range<usize>, element_bounds: Bounds<Pixels>,
        window: &mut Window, cx: &mut App,
    ) -> Option<Bounds<Pixels>>;

    /// Hit testing: which character is at this pixel position?
    fn character_index_for_point(
        &mut self, point: Point<Pixels>, window: &mut Window, cx: &mut App,
    ) -> Option<usize>;
}
```

The critical line that connects IME to your element -- must be called in `paint`:
```rust
window.handle_input(
    &focus_handle,
    ElementInputHandler::new(bounds, editor_entity.clone()),
    cx,
);
```

Without this call, no keyboard input reaches your editor. It must happen in `paint`, not `prepaint`.

For multi-cursor editors, `replace_text_in_range` must fan out the edit to all cursors. The Zed editor tracks `ime_transaction` to group all composition edits into one undo step.

### Option 2: Build a Simple Text Input from Scratch

For single-line inputs, search boxes, or rename fields, the full editor is overkill. See `crates/gpui/examples/input.rs` (~650 lines) for the minimal pattern:

```rust
struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    selected_range: Range<usize>,       // byte offsets
    marked_range: Option<Range<usize>>, // IME composition range
    last_layout: Option<ShapedLine>,    // cached for hit testing
    is_selecting: bool,                 // mouse drag state
}
```

The custom `TextElement` implements `Element` directly:

**prepaint:**
1. Get content string and selection range
2. Build `TextRun` array (font, color; underline for IME marked text)
3. `window.text_system().shape_line(text, font_size, &runs, None)` → `ShapedLine`
4. `shaped_line.x_for_index(cursor_offset)` → cursor pixel X position
5. Build `PaintQuad` for cursor (2px wide bar) and selection (colored rectangle)

**paint:**
1. `window.handle_input(&focus_handle, handler, cx)` -- connect IME (must be first)
2. Paint selection quad
3. `shaped_line.paint(origin, line_height, ...)` -- draw the text
4. Paint cursor quad (only when focused)
5. Cache layout for hit testing

**Hit testing (click to place cursor):**
```rust
shaped_line.closest_index_for_x(click_x - text_origin_x) // → byte offset
```

**Cursor positioning:**
```rust
shaped_line.x_for_index(byte_offset) // → pixel X
```

The `Render` impl wires it together:
```rust
impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("TextInput")
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::select_all))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(TextElement { input: cx.entity() })
    }
}
```

### Key GPUI Text APIs

```rust
// Shape a line of text with styled runs:
let shaped_line = window.text_system().shape_line(
    text,           // &str
    font_size,      // Pixels
    &runs,          // &[TextRun] -- each run has font, color, len
    wrap_width,     // Option<Pixels>
)?;

// Paint shaped text:
shaped_line.paint(
    origin,         // Point<Pixels> -- top-left
    line_height,    // Pixels
    align,          // TextAlign
    wrap,           // Option wrapping state
    window,
    cx,
)?;

// Cursor positioning (byte offset → pixel X):
let x: Pixels = shaped_line.x_for_index(byte_offset);

// Hit testing (pixel X → byte offset):
let offset: usize = shaped_line.closest_index_for_x(x);

// Draw a colored rectangle (for cursors, selections, backgrounds):
window.paint_quad(fill(bounds, color));
window.paint_quad(outline(bounds, border_color));

// Clip rendering to a region:
window.with_content_mask(Some(ContentMask { bounds }), |window| {
    // Everything painted here is clipped to bounds
});

// Register for keyboard/IME input (must be in paint phase):
window.handle_input(&focus_handle, ElementInputHandler::new(bounds, entity), cx);
```

### Standalone App: Theme Initialization

If you're building an app outside the Zed workspace that uses the `editor` crate, you need to initialize the theme system at startup:

```rust
fn main() {
    application().run(|cx: &mut App| {
        // Required: initialize theme system
        theme::init(cx);
        theme_settings::init(cx);

        // Required: initialize settings
        settings::init(cx);

        // Required: initialize language registry (for tree-sitter)
        let language_registry = Arc::new(LanguageRegistry::new(cx.background_executor().clone()));
        // Register languages you need:
        language_registry.register_native_grammars([("rust", tree_sitter_rust::LANGUAGE)]);

        // Open window with editor
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| {
                let buffer = cx.new(|cx| language::Buffer::local("fn main() {}", cx));
                let multi_buffer = cx.new(|cx| MultiBuffer::singleton(buffer, cx));
                Editor::for_multibuffer(multi_buffer, None, true, window, cx)
            })
        });
    });
}
```

The editor will use whatever theme is active. The built-in fallback is "One Dark". Load custom themes from JSON using `ThemeRegistry::load_user_theme()`.
