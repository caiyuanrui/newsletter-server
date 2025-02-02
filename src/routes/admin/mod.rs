mod dashboard;
mod logout;
mod password;

pub use dashboard::admin_dashboard;
pub use logout::log_out;
pub use password::FormData as PasswordFormData;
pub use password::{change_password, change_password_form};
