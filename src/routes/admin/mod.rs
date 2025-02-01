mod dashboard;
mod password;

pub use dashboard::admin_dashboard;
pub use password::FormData as PasswordFormData;
pub use password::{change_password, change_password_form};
