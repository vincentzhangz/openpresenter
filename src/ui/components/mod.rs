/// Reusable UI components providing a consistent look across all panels.
///
/// Each sub-module exposes pure view functions returning `Element<'_, Message>`.
/// Import symbols directly or use the flat re-exports at the bottom of this file.
pub mod add_button;
pub mod color_slider;
pub mod dialog;
pub mod divider;
pub mod empty_state;
pub mod error_toast;
pub mod group_color;
pub mod labeled_field;
pub mod list_item;
pub mod live_badge;
pub mod preview_text;
pub mod reorder_toolbar;
pub mod search_input;
pub mod section_header;
pub mod tab_button;
pub mod toggle_button;

// Flatten commonly-used items for convenient glob imports.
pub use add_button::add_button;
pub use color_slider::{color_channel_slider, color_swatch_btn};
pub use dialog::{confirm_dialog, dialog_overlay};
pub use divider::divider;
pub use empty_state::empty_state;
pub use error_toast::error_toast;
pub use group_color::{GROUP_COLORS, group_color, group_label_widget, group_option_color};
pub use labeled_field::field_col;
pub use list_item::list_item;
pub use live_badge::live_badge;
pub use preview_text::truncate;
pub use reorder_toolbar::{ReorderControls, reorder_toolbar};
pub use search_input::search_input;
pub use section_header::{section_header, section_label};
pub use tab_button::{tab_bar, tab_btn};
pub use toggle_button::{compact_toggle_btn, option_btn, toggle_btn};
