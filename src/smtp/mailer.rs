use askama::Template;
use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::CONFIG;
use crate::send::send::SendInfo;

/// SMTP 邮件发送器
pub struct SmtpMailer;

impl SmtpMailer {
    /// 创建新的 SmtpMailer 实例
    pub fn new() -> Self {
        Self
    }

    /// 发送邮件
    pub fn send(&self, send_info: SendInfo) -> Result<String, String> {
        let smtp_send_template = SmtpSendTemplate {
            title: send_info.send_title.clone(),
            from: send_info.from_email.clone(),
            action_url: send_info.foward_url.clone(),
            message: send_info.send_data.clone(),
        };

        // 渲染 HTML 模板
        let send_html_text = smtp_send_template.render().map_err(|e| {
            eprintln!("❌ 模板渲染失败: {}", e);
            eprintln!("   标题: {}", smtp_send_template.title);
            eprintln!("   发件人: {}", smtp_send_template.from);
            format!("模板渲染失败: {}", e)
        })?;

        // 构建邮件
        let email = Message::builder()
            .from(
                Mailbox::new(
                    Some(send_info.user.to_owned()),
                    send_info.user_email.parse().unwrap(),
                )
            )
            .to(
                Mailbox::new(
                    Some(send_info.send_to_user.to_owned()),
                    send_info.send_to_email.parse().unwrap(),
                )
            )
            .subject(&send_info.send_title)
            .header(ContentType::TEXT_HTML)
            .body(send_html_text)
            .map_err(|e| format!("邮件构建失败: {}", e))?;

        // 创建 SMTP 传输
        let creds = Credentials::new(CONFIG.smtp_email.clone(), CONFIG.email_password.clone());
        let mailer = SmtpTransport::relay(CONFIG.smtp_server.as_str())
            .map_err(|e| format!("SMTP 服务器连接失败: {}", e))?
            .credentials(creds)
            .build();

        // 发送邮件
        mailer.send(&email)
            .map(|_| "发送成功".to_string())
            .map_err(|e| format!("SMTP 发送失败: {}", e))
    }
}

impl Default for SmtpMailer {
    fn default() -> Self {
        Self::new()
    }
}

/// 邮件模板结构
#[derive(Template)]
#[template(path = "email.html")]
struct SmtpSendTemplate {
    title: String,
    from: String,
    action_url: String,
    message: String,
}
