use crate::{
    ChangedReport, DocsUnderReport, ListStateReport, StatusReport,
};

pub fn print_status_json(report: &StatusReport) {
    let json = serde_json::to_string_pretty(report).unwrap_or_else(|e| {
        format!("{{\"error\": \"{}\"}}", e)
    });
    println!("{}", json);
}

pub fn print_changed_json(report: &ChangedReport) {
    let json = serde_json::to_string_pretty(report).unwrap_or_else(|e| {
        format!("{{\"error\": \"{}\"}}", e)
    });
    println!("{}", json);
}

pub fn print_list_json(report: &ListStateReport) {
    let json = serde_json::to_string_pretty(report).unwrap_or_else(|e| {
        format!("{{\"error\": \"{}\"}}", e)
    });
    println!("{}", json);
}

pub fn print_docs_under_json(report: &DocsUnderReport) {
    let json = serde_json::to_string_pretty(report).unwrap_or_else(|e| {
        format!("{{\"error\": \"{}\"}}", e)
    });
    println!("{}", json);
}
