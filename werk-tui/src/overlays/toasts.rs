use ftui::Frame;
use ftui::layout::Rect;
use ftui::style::Style;
use ftui::widgets::Widget;
use ftui::widgets::toast::{Toast as FtuiToast, ToastIcon, ToastStyle as FtuiToastStyle};

use werk_shared::truncate;

use crate::app::WerkApp;
use crate::theme::*;
use crate::types::{MAX_VISIBLE_TOASTS, ToastSeverity};

impl WerkApp {
    pub(crate) fn render_toasts(&self, area: Rect, frame: &mut Frame<'_>) {
        if self.toasts.is_empty() {
            return;
        }

        let visible_toasts: Vec<_> = self
            .toasts
            .iter()
            .rev()
            .take(MAX_VISIBLE_TOASTS)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        for (i, toast) in visible_toasts.iter().enumerate() {
            let toast_width = (toast.message.len() as u16 + 6).min(area.width.saturating_sub(2));
            let x = area.width.saturating_sub(toast_width + 1);
            let y = 1 + (i as u16);

            if y >= area.height.saturating_sub(2) {
                break;
            }

            let toast_area = Rect::new(x, y, toast_width, 1);
            let msg = truncate(&toast.message, toast_width.saturating_sub(4) as usize).to_string();

            let (icon, style_variant) = match toast.severity {
                ToastSeverity::Info => (ToastIcon::Info, FtuiToastStyle::Info),
                ToastSeverity::Warning => (ToastIcon::Warning, FtuiToastStyle::Warning),
                ToastSeverity::Alert => (ToastIcon::Error, FtuiToastStyle::Error),
            };

            let ftui_toast = FtuiToast::new(msg)
                .icon(icon)
                .style_variant(style_variant)
                .style(Style::new().fg(toast.color()).bg(CLR_BG_DARK).bold());
            ftui_toast.render(toast_area, frame);
        }
    }
}
