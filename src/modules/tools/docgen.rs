// Document Generator - PDF generation for order forms and proposals
// Both use HTML template + Chrome headless for professional formatting

use crate::core::error::{CmdResult, CommandError};
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

// ============================================================================
// Types - Order Form
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrderFormData {
    // Order details
    pub order_form_reference: String,
    pub agreement_reference: String,

    // Customer info
    pub customer_name: String,
    pub customer_uen: String,
    pub customer_address: String,

    // Contact
    pub contact_name: String,
    pub contact_phone: String,
    pub contact_email: String,

    // Subscription
    pub subscription_start_date: String,
    pub subscription_end_date: String,
    pub subscription_fee: String,
    pub service_term: String,
    pub billing_cycle: String,
    pub annual_subscription_fee: String,

    // Entitlement
    pub feature_plan: String,
    pub solutions: String,
    pub number_of_outlets: String,
    pub solution_name: String,
    pub scope_items: Vec<String>,
    pub complementary_items: Vec<String>,
    pub implementation_plan: Vec<String>,

    // Payments
    pub subscription_payments: Vec<PaymentRow>,
    pub implementation_payments: Vec<ImplementationPaymentRow>,
    pub implementation_fee: String,
    pub total_contract_value: String,

    // Customer officer
    pub customer_officer_name: String,
    pub customer_officer_title: String,
    pub customer_officer_email: String,
    pub customer_officer_phone: String,

    // Agreement
    pub agreement_day: String,
    pub agreement_month: String,
    pub agreement_year: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaymentRow {
    pub period: String,
    pub date: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImplementationPaymentRow {
    pub sn: String,
    pub milestone: String,
    pub percentage: String,
    pub date: String,
    pub amount: String,
}


// ============================================================================
// Parser - Extract data from markdown files
// ============================================================================

/// Parse order form markdown file into structured data
pub fn parse_order_form_markdown(markdown: &str) -> CmdResult<OrderFormData> {
    let mut data = OrderFormData::default();
    let yaml_values = extract_yaml_values(markdown);

    // Order details
    data.order_form_reference = yaml_values
        .get("orderFormReference")
        .cloned()
        .unwrap_or_default();
    data.agreement_reference = yaml_values
        .get("agreementReference")
        .cloned()
        .unwrap_or_default();

    // Customer info
    data.customer_name = yaml_values
        .get("customerName")
        .cloned()
        .unwrap_or_default();
    data.customer_uen = yaml_values.get("customerUEN").cloned().unwrap_or_default();
    data.customer_address = yaml_values
        .get("customerAddress")
        .cloned()
        .unwrap_or_default();

    // Contact
    data.contact_name = yaml_values.get("contactName").cloned().unwrap_or_default();
    data.contact_phone = yaml_values
        .get("contactPhone")
        .cloned()
        .unwrap_or_default();
    data.contact_email = yaml_values
        .get("contactEmail")
        .cloned()
        .unwrap_or_default();

    // Subscription
    data.subscription_start_date = yaml_values
        .get("subscriptionStartDate")
        .cloned()
        .unwrap_or_default();
    data.subscription_end_date = yaml_values
        .get("subscriptionEndDate")
        .cloned()
        .unwrap_or_default();
    data.subscription_fee = yaml_values
        .get("subscriptionFee")
        .cloned()
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    data.service_term = yaml_values.get("serviceTerm").cloned().unwrap_or_default();
    data.billing_cycle = yaml_values.get("billingCycle").cloned().unwrap_or_default();
    data.annual_subscription_fee = yaml_values
        .get("annualSubscriptionFee")
        .cloned()
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    // Entitlement
    data.feature_plan = yaml_values.get("featurePlan").cloned().unwrap_or_default();
    data.solutions = yaml_values.get("solutions").cloned().unwrap_or_default();
    data.number_of_outlets = yaml_values
        .get("numberOfOutlets")
        .cloned()
        .unwrap_or_default();
    data.solution_name = yaml_values
        .get("solutionName")
        .cloned()
        .unwrap_or_default();

    // Scope items (from markdown list under "### Scope Items")
    data.scope_items = extract_list_items(markdown, "Scope Items");
    data.complementary_items = extract_list_items(markdown, "Complementary Items");
    data.implementation_plan = extract_list_items(markdown, "Implementation Plan");

    // Subscription payments (from markdown table)
    data.subscription_payments = extract_subscription_payments(markdown);

    // Implementation payments (from markdown table)
    data.implementation_payments = extract_implementation_payments(markdown);
    data.implementation_fee = yaml_values
        .get("implementationFee")
        .cloned()
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    // Auto-calculate percentages if not provided
    let impl_fee: f64 = data.implementation_fee.replace(',', "").parse().unwrap_or(0.0);
    if impl_fee > 0.0 {
        for p in &mut data.implementation_payments {
            if p.percentage.is_empty() {
                let amount: f64 = p.amount.replace(',', "").parse().unwrap_or(0.0);
                let pct = (amount / impl_fee) * 100.0;
                if (pct - pct.round()).abs() < 0.01 {
                    p.percentage = format!("{}%", pct.round() as i64);
                } else {
                    p.percentage = format!("{:.1}%", pct);
                }
            }
        }
    }
    data.total_contract_value = yaml_values
        .get("totalContractValue")
        .cloned()
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    // Customer officer
    data.customer_officer_name = yaml_values
        .get("customerOfficerName")
        .cloned()
        .unwrap_or_default();
    data.customer_officer_title = yaml_values
        .get("customerOfficerTitle")
        .cloned()
        .unwrap_or_default();
    data.customer_officer_email = yaml_values
        .get("customerOfficerEmail")
        .cloned()
        .unwrap_or_default();
    data.customer_officer_phone = yaml_values
        .get("customerOfficerPhone")
        .cloned()
        .unwrap_or_default();

    // Agreement
    data.agreement_day = yaml_values
        .get("agreementDay")
        .cloned()
        .unwrap_or_else(|| "____".to_string());
    data.agreement_month = yaml_values
        .get("agreementMonth")
        .cloned()
        .unwrap_or_default();
    data.agreement_year = yaml_values
        .get("agreementYear")
        .cloned()
        .unwrap_or_default();

    // Validation: check for common data mismatches
    let sub_fee: u64 = data.subscription_fee.replace(',', "").parse().unwrap_or(0);
    let sub_total: u64 = data.subscription_payments.iter()
        .filter_map(|p| p.amount.replace(',', "").parse::<u64>().ok())
        .sum();
    let term_years: u64 = data.service_term.chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse().unwrap_or(1);

    if sub_total > 0 && sub_fee > 0 {
        // Check: fee * years should equal payment total
        if sub_fee * term_years != sub_total {
            eprintln!("[docgen] WARNING: subscriptionFee ({}) x {} years = {}, but payment rows sum to {}. Verify the data.",
                data.subscription_fee, term_years, sub_fee * term_years, sub_total);
        }
    }

    let impl_total: u64 = data.implementation_payments.iter()
        .filter_map(|p| p.amount.replace(',', "").parse::<u64>().ok())
        .sum();
    let impl_fee_val: u64 = data.implementation_fee.replace(',', "").parse().unwrap_or(0);
    if impl_total > 0 && impl_fee_val > 0 && impl_total != impl_fee_val {
        eprintln!("[docgen] WARNING: implementationFee ({}) != implementation payment rows sum ({}). Verify the data.",
            data.implementation_fee, impl_total);
    }

    Ok(data)
}

/// Extract all YAML key-value pairs from markdown code blocks
fn extract_yaml_values(markdown: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();

    // Match ```yaml ... ``` blocks
    let yaml_block_re = Regex::new(r"```yaml\s*\n([\s\S]*?)\n```").unwrap();

    for caps in yaml_block_re.captures_iter(markdown) {
        let yaml_content = caps.get(1).map(|m| m.as_str()).unwrap_or("");

        for line in yaml_content.lines() {
            let line = line.trim();
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim().trim_matches('"');
                if !key.is_empty() && !value.is_empty() {
                    values.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    values
}

/// Extract list items under a specific heading (handles sub-headings like **Title:**)
/// Convert **bold** markdown syntax to <strong> HTML tags
fn convert_inline_bold(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume second *
            let mut bold_text = String::new();
            let mut closed = false;
            while let Some(bc) = chars.next() {
                if bc == '*' && chars.peek() == Some(&'*') {
                    chars.next(); // consume closing **
                    closed = true;
                    break;
                }
                bold_text.push(bc);
            }
            if closed {
                result.push_str(&format!("<strong>{}</strong>", bold_text));
            } else {
                result.push_str("**");
                result.push_str(&bold_text);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn extract_list_items(markdown: &str, heading: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut in_section = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Check if we found the heading
        if trimmed.starts_with("### ") && trimmed.contains(heading) {
            in_section = true;
            continue;
        }

        // Check if we've left the section (another heading or separator)
        if in_section && (trimmed.starts_with("## ") || trimmed.starts_with("### ") || trimmed == "---") {
            break;
        }

        // Extract bold sub-headings like **AR Automation:** or **Phase 1: Title (Weeks 1-2)**
        if in_section && trimmed.starts_with("**") && trimmed.ends_with("**") && !trimmed.starts_with("- ") {
            // Remove ** markers and optional trailing colon
            let sub_heading = trimmed.trim_start_matches("**").trim_end_matches("**");
            let sub_heading = sub_heading.trim_end_matches(':');
            items.push(format!("__SUBHEADING__{}", sub_heading));
            continue;
        }

        // Extract list items
        if in_section && trimmed.starts_with("- ") {
            items.push(trimmed[2..].to_string());
        }
    }

    items
}

/// Extract subscription payments from markdown table
fn extract_subscription_payments(markdown: &str) -> Vec<PaymentRow> {
    let mut payments = Vec::new();
    let mut in_section = false;
    let mut in_table = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Check if we found the section
        if trimmed.starts_with("## ") && trimmed.contains("Subscription Payments") {
            in_section = true;
            continue;
        }

        // Check if we've left the section
        if in_section && (trimmed.starts_with("## ") || trimmed == "---") && !trimmed.contains("Subscription Payments") {
            break;
        }

        // Parse table rows
        if in_section && trimmed.starts_with('|') && trimmed.ends_with('|') {
            if trimmed.contains("---") {
                in_table = true;
                continue;
            }
            if in_table {
                let cells: Vec<&str> = trimmed
                    .split('|')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim())
                    .collect();

                if cells.len() >= 3 {
                    payments.push(PaymentRow {
                        period: cells[0].to_string(),
                        date: cells[1].to_string(),
                        amount: cells[2].replace(',', "").to_string(),
                    });
                }
            }
        }
    }

    payments
}

/// Extract implementation payments from markdown table
fn extract_implementation_payments(markdown: &str) -> Vec<ImplementationPaymentRow> {
    let mut payments = Vec::new();
    let mut in_section = false;
    let mut in_table = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Check if we found the section
        if trimmed.starts_with("## ") && trimmed.contains("Implementation Payments") {
            in_section = true;
            continue;
        }

        // Check if we've left the section
        if in_section && (trimmed.starts_with("## ") || trimmed == "---" || trimmed.starts_with("```")) && !trimmed.contains("Implementation Payments") {
            break;
        }

        // Parse table rows
        if in_section && trimmed.starts_with('|') && trimmed.ends_with('|') {
            if trimmed.contains("---") {
                in_table = true;
                continue;
            }
            if in_table {
                let cells: Vec<&str> = trimmed
                    .split('|')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim())
                    .collect();

                if cells.len() >= 5 {
                    // 5-column format: S/N | Milestone | % | Date | Amount
                    payments.push(ImplementationPaymentRow {
                        sn: cells[0].to_string(),
                        milestone: cells[1].to_string(),
                        percentage: cells[2].to_string(),
                        date: cells[3].to_string(),
                        amount: cells[4].replace(',', "").to_string(),
                    });
                } else if cells.len() >= 4 {
                    // 4-column format: S/N | Milestone | Date | Amount
                    // percentage will be calculated after parsing
                    payments.push(ImplementationPaymentRow {
                        sn: cells[0].to_string(),
                        milestone: cells[1].to_string(),
                        percentage: String::new(),
                        date: cells[2].to_string(),
                        amount: cells[3].replace(',', "").to_string(),
                    });
                }
            }
        }
    }

    payments
}

// ============================================================================
// PDF Generator - Order Form (HTML + Chrome headless)
// ============================================================================

/// Generate order form PDF from data using HTML template + Chrome headless
pub fn generate_order_form_pdf(data: &OrderFormData, output_path: &str) -> CmdResult<String> {
    // Generate HTML from data
    let full_html = wrap_in_order_form_template(data);

    // Write HTML to temp file
    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join(format!("order_form_{}.html", std::process::id()));
    let mut html_file = fs::File::create(&html_path)?;
    html_file.write_all(full_html.as_bytes())?;

    // Convert HTML to PDF using Chrome headless
    let chrome_path = find_chrome_path()?;

    let status = Command::new(&chrome_path)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--no-pdf-header-footer",
            &format!("--print-to-pdf={}", output_path),
            &html_path.to_string_lossy(),
        ])
        .status()?;

    // Clean up temp file
    let _ = fs::remove_file(&html_path);

    if !status.success() {
        return Err(CommandError::Internal("Chrome PDF generation failed".to_string()));
    }

    Ok(output_path.to_string())
}

/// Wrap order form data in HTML template
fn wrap_in_order_form_template(data: &OrderFormData) -> String {
    // Generate scope items HTML with sub-headings support
    let mut scope_items_html = String::new();
    let mut in_list = false;

    for item in &data.scope_items {
        if item.starts_with("__SUBHEADING__") {
            // Close previous list if open
            if in_list {
                scope_items_html.push_str("      </ul>\n");
                in_list = false;
            }
            // Add sub-heading
            let heading = item.trim_start_matches("__SUBHEADING__");
            scope_items_html.push_str(&format!("      <p class=\"scope-title\"><strong>{}</strong></p>\n", heading));
        } else {
            // Start list if not already in one
            if !in_list {
                scope_items_html.push_str("      <ul>\n");
                in_list = true;
            }
            // Convert **bold** markdown to <strong> tags
            let converted = convert_inline_bold(item);
            scope_items_html.push_str(&format!("        <li>{}</li>\n", converted));
        }
    }
    // Close final list if open
    if in_list {
        scope_items_html.push_str("      </ul>");
    }

    // Generate complementary items HTML
    let complementary_html = if !data.complementary_items.is_empty() {
        let items: String = data.complementary_items
            .iter()
            .map(|item| format!("        <li>{}</li>", item))
            .collect::<Vec<_>>()
            .join("\n");
        format!(r#"
      <p class="scope-title"><strong>Complementary</strong></p>
      <ul>
{}
      </ul>"#, items)
    } else {
        String::new()
    };

    // Generate implementation plan HTML with sub-headings support
    let implementation_plan_html = if !data.implementation_plan.is_empty() {
        let mut plan_html = String::new();
        let mut in_list = false;

        for item in &data.implementation_plan {
            if item.starts_with("__SUBHEADING__") {
                if in_list {
                    plan_html.push_str("      </ul>\n");
                    in_list = false;
                }
                let heading = item.trim_start_matches("__SUBHEADING__");
                plan_html.push_str(&format!("      <p class=\"scope-title\"><strong>{}</strong></p>\n", heading));
            } else {
                if !in_list {
                    plan_html.push_str("      <ul>\n");
                    in_list = true;
                }
                plan_html.push_str(&format!("        <li>{}</li>\n", item));
            }
        }
        if in_list {
            plan_html.push_str("      </ul>");
        }

        format!(r#"
    <div class="entitlement-section">
      <p class="section-title"><strong>Implementation Plan:</strong></p>
{}
    </div>"#, plan_html)
    } else {
        String::new()
    };

    // Build subscription fee description (dynamic based on billing cycle)
    let subscription_fee_desc = if !data.billing_cycle.is_empty() && !data.annual_subscription_fee.is_empty() {
        format!(
            "SGD${fee} per annum ({cycle} billing, exclusive of any GST imposed in Singapore), payable in advance before the start of each period. Details of the payment schedule is attached in the order form hereto (&quot;Subscription Fee Payment Schedule&quot;). Customer may opt for annual billing at SGD${annual}/year by notifying VAL at least 1 month before the current quarter ends, effective from the following quarter.",
            fee = data.subscription_fee,
            cycle = data.billing_cycle.to_lowercase(),
            annual = data.annual_subscription_fee,
        )
    } else {
        format!(
            "SGD${fee} per annum (exclusive of any GST imposed in Singapore), payable in advance before the start of each period. Details of the payment schedule is attached in the order form hereto (&quot;Subscription Fee Payment Schedule&quot;)",
            fee = data.subscription_fee,
        )
    };

    // Conditionally render outlets line
    let outlets_line = if data.number_of_outlets.is_empty() {
        String::new()
    } else {
        format!("      <p>Number of Outlets: {}</p>", data.number_of_outlets)
    };

    // Generate subscription payments rows and compute total from row amounts
    let sub_payments_html: String = data.subscription_payments
        .iter()
        .map(|p| format!(r#"        <tr>
          <td>{}</td>
          <td>{}</td>
          <td>SGD${}</td>
        </tr>"#, p.period, p.date, p.amount))
        .collect::<Vec<_>>()
        .join("\n");

    let subscription_total: u64 = data.subscription_payments
        .iter()
        .filter_map(|p| p.amount.replace(",", "").parse::<u64>().ok())
        .sum();
    let subscription_total_str = format!("{}", subscription_total.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(","));

    // Generate implementation payments rows
    let impl_payments_html: String = data.implementation_payments
        .iter()
        .map(|p| format!(r#"        <tr>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td>SGD${}</td>
        </tr>"#, p.sn, p.milestone, p.percentage, p.date, p.amount))
        .collect::<Vec<_>>()
        .join("\n");

    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Software As A Service (SAAS) Order Form</title>
  <style>
    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }}

    body {{
      font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
      font-size: 11pt;
      line-height: 1.4;
      color: #333;
      background: white;
    }}

    .document {{
      max-width: 210mm;
      padding: 15mm 20mm;
      margin: 0 auto;
      background: white;
    }}

    .page-break {{
      page-break-before: always;
      margin-top: 30px;
    }}

    .header {{
      margin-bottom: 10px;
      border-bottom: 3px solid #00A0E3;
      padding-bottom: 10px;
    }}

    .logo {{
      text-align: left;
    }}

    .title {{
      font-size: 24pt;
      font-weight: bold;
      color: #333;
      margin: 20px 0 10px 0;
    }}

    h2 {{
      font-size: 14pt;
      font-weight: bold;
      color: #00A0E3;
      margin: 25px 0 10px 0;
      border-bottom: 2px solid #00A0E3;
      padding-bottom: 5px;
    }}

    h3 {{
      font-size: 12pt;
      font-weight: bold;
      color: #333;
      margin: 10px 0 5px 0;
    }}

    .reference {{
      font-size: 10pt;
      margin-bottom: 15px;
    }}

    .highlight {{
      padding: 1px 3px;
    }}

    table {{
      width: 100%;
      border-collapse: collapse;
      margin: 10px 0;
    }}

    .info-table td {{
      padding: 5px 10px;
      border: 1px dotted #ccc;
      vertical-align: top;
    }}

    .info-table .label {{
      font-weight: normal;
      color: #666;
      width: 20%;
    }}

    .info-table .value {{
      font-weight: normal;
      width: 30%;
    }}

    .payment-table {{
      border: 1px solid #ddd;
    }}

    .payment-table th {{
      background-color: #00A0E3;
      color: white;
      padding: 10px;
      text-align: left;
      font-weight: bold;
    }}

    .payment-table td {{
      padding: 8px 10px;
      border-bottom: 1px dotted #ddd;
    }}

    .payment-table .total-row td {{
      border-top: 2px solid #333;
      font-weight: bold;
    }}

    .details-table {{
      border: 1px dotted #ccc;
    }}

    .details-table td {{
      padding: 10px;
      vertical-align: top;
    }}

    .details-left {{
      width: 70%;
      border-right: 1px dotted #ccc;
    }}

    .details-right {{
      width: 30%;
    }}

    .services-box {{
      padding: 10px 0;
      margin: 10px 0;
      border-top: 1px dotted #ccc;
      border-bottom: 1px dotted #ccc;
    }}

    .entitlement-section {{
      margin: 15px 0;
      padding: 10px 0 10px 15px;
      border-left: 3px solid #00A0E3;
    }}

    .entitlement-section p {{
      margin-bottom: 5px;
    }}

    .entitlement-section .section-title {{
      margin-bottom: 10px;
    }}

    .entitlement-section .scope-title {{
      margin-top: 15px;
      margin-bottom: 5px;
    }}

    .entitlement-section ul {{
      margin-left: 25px;
      margin-top: 5px;
      margin-bottom: 10px;
    }}

    .entitlement-section li {{
      margin-bottom: 4px;
    }}

    .small {{
      font-size: 9pt;
      color: #666;
      margin-top: 10px;
    }}

    .info-box {{
      border: 1px dotted #ccc;
      padding: 10px 15px;
      margin: 10px 0;
    }}

    .info-box p {{
      margin-bottom: 5px;
    }}

    .notices-section {{
      page-break-inside: avoid;
    }}

    .notices-section > p {{
      margin-bottom: 10px;
    }}

    .notices-grid {{
      display: flex;
      gap: 20px;
      margin-top: 15px;
    }}

    .notice-box {{
      flex: 1;
      border: 1px solid #ddd;
      padding: 15px;
    }}

    .notice-box h3 {{
      color: #00A0E3;
      margin-bottom: 10px;
    }}

    .notice-box p {{
      margin-bottom: 5px;
      font-size: 10pt;
    }}

    .agreement-text {{
      margin: 20px 0;
      text-align: justify;
    }}

    .signature-section {{
      display: flex;
      gap: 40px;
      margin-top: 60px;
      page-break-inside: avoid;
    }}

    .signature-box {{
      flex: 1;
    }}

    .signature-box h3 {{
      margin-bottom: 30px;
    }}

    .signature-box p {{
      margin-bottom: 15px;
    }}

    .signature-line {{
      margin-top: 40px;
    }}

    .terms-box {{
      border: 1px solid #ddd;
      padding: 15px 20px;
      margin: 25px 0;
      text-align: justify;
    }}

    .terms-box p {{
      margin-bottom: 12px;
    }}

    .terms-box p:last-child {{
      margin-bottom: 0;
    }}

    .terms-box a {{
      color: #00A0E3;
      text-decoration: underline;
    }}

    .customer-signature {{
      margin-top: 30px;
      page-break-inside: avoid;
    }}

    .customer-signature h3 {{
      font-size: 12pt;
      font-weight: bold;
      margin-bottom: 25px;
    }}

    .signature-grid {{
      display: flex;
      gap: 40px;
    }}

    .sig-left, .sig-right {{
      flex: 1;
    }}

    .signature-grid p {{
      margin-bottom: 12px;
    }}

    @page {{
      size: A4;
      margin: 15mm 20mm;
    }}

    @media print {{
      .document {{
        padding: 0;
      }}
    }}
  </style>
</head>
<body>
  <div class="document">
    <!-- Header with Logo -->
    <header class="header">
      <div class="logo">
        <img src="data:image/png;base64,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" alt="ThinkVAL" height="50">
      </div>
    </header>

    <!-- Title -->
    <h1 class="title">Software As A Service (SAAS) Order Form</h1>

    <p class="reference">Order Form Reference Number: <strong>{order_form_reference}</strong></p>

    <!-- Customer Info Table -->
    <table class="info-table">
      <tr>
        <td class="label">Customer:</td>
        <td class="value highlight">{customer_name}</td>
        <td class="label">Contact:</td>
        <td class="value">{contact_name}</td>
      </tr>
      <tr>
        <td class="label">UEN:</td>
        <td class="value">{customer_uen}</td>
        <td class="label">Phone:</td>
        <td class="value">{contact_phone}</td>
      </tr>
      <tr>
        <td class="label">Address:</td>
        <td class="value" colspan="1">{customer_address}</td>
        <td class="label">Email:</td>
        <td class="value">{contact_email}</td>
      </tr>
      <tr>
        <td class="label">Subscription Start Date:</td>
        <td class="value">{subscription_start_date}</td>
        <td class="label">Subscription End Date:</td>
        <td class="value">{subscription_end_date}</td>
      </tr>
    </table>

    <!-- Services Description -->
    <div class="services-box">
      <p><strong>Services:</strong> 'VAL' Value Analytics Layer (VAL) is an all in one data automation platform unifying all data capabilities under one roof, designed for non-tech users along with an extensive ecosystem integration. VAL provides data collection, data processing and data visualisation capabilities.<br>(the "Service(s)").</p>
    </div>

    <!-- Subscription Details Table -->
    <table class="details-table">
      <tr>
        <td class="details-left">
          <strong>Subscription Fee:</strong> {subscription_fee_desc}
        </td>
        <td class="details-right">
          <strong>Service Term:</strong> {service_term}
        </td>
      </tr>
    </table>

    <!-- Subscription Entitlement -->
    <div class="entitlement-section">
      <p class="section-title"><strong>Subscription Entitlement:</strong></p>
      <p>Solutions: <span class="highlight">{solutions}</span></p>
{outlets_line}
      <p>Unlimited Users</p>

      <p class="scope-title"><strong>Scope:</strong></p>
{scope_items}
{complementary}
    </div>

    <!-- Implementation Service -->
    <div class="info-box">
      <p><strong>Implementation Service:</strong> Company will use commercially reasonable efforts to provide Customer the services described in the Statement of Work ("SOW") attached as Appendix C hereto ("Implementation Services"), and Customer shall pay Company the Implementation Fee in accordance with the terms herein</p>
      <p><strong>Implementation Fee (one-time):</strong> SGD${implementation_fee} (exclusive of any GST imposed in Singapore)</p>
    </div>

    <!-- Implementation Plan -->
{implementation_plan}

    <!-- Fee Payment Schedule -->
    <h2>Fee Payment Schedule</h2>

    <table class="payment-table">
      <thead>
        <tr>
          <th>Payment</th>
          <th>Payment Date</th>
          <th>Amount</th>
        </tr>
      </thead>
      <tbody>
{subscription_payments}
        <tr class="total-row">
          <td colspan="2"><strong>Total</strong></td>
          <td><strong>SGD${subscription_total}</strong></td>
        </tr>
      </tbody>
    </table>

    <h2>Implementation Fee Payment Schedule</h2>

    <table class="payment-table">
      <thead>
        <tr>
          <th>S/N</th>
          <th>Payment Milestones</th>
          <th>% of SOW Total Contract Price</th>
          <th>Payment Date</th>
          <th>Contract Price (SGD)</th>
        </tr>
      </thead>
      <tbody>
{implementation_payments}
        <tr class="total-row">
          <td colspan="2"><strong>Total</strong></td>
          <td><strong>100%</strong></td>
          <td></td>
          <td><strong>SGD${implementation_fee}</strong></td>
        </tr>
      </tbody>
    </table>

    <!-- Terms and Signature Section -->
    <div class="terms-box">
      <p>This Agreement shall automatically renew annually, unless either party provides the other party written notice of termination at least sixty (60) days in advance of the next renewal date, in which case the Agreement shall terminate as of such next renewal date. The 60 days' notice period shall apply also if Customer wants to downgrade their subscription, including but not limited to reducing the number of users, accounts, or products.</p>
      <p>During the Term, ThinkVAL may include Customer's name and logo in ThinkVAL website, press releases, promotional and sales literature, and lists of customers.</p>
      <p>By signing this Order Form, the Customer agrees to ThinkVAL Terms Of Service published at: <a href="https://www.thinkval.com/legal/terms-of-service">https://www.thinkval.com/legal/terms-of-service</a></p>
    </div>

    <div class="customer-signature">
      <h3>Customer Signature:</h3>
      <div class="signature-grid">
        <div class="sig-left">
          <p>Signature: _______________________</p>
          <p>Name: {customer_officer_name}</p>
        </div>
        <div class="sig-right">
          <p>Title: {customer_officer_title}</p>
          <p>Date:</p>
        </div>
      </div>
    </div>
  </div>
</body>
</html>"##,
        order_form_reference = data.order_form_reference,
        customer_name = data.customer_name,
        customer_uen = data.customer_uen,
        customer_address = data.customer_address,
        contact_name = data.contact_name,
        contact_phone = data.contact_phone,
        contact_email = data.contact_email,
        subscription_start_date = data.subscription_start_date,
        subscription_end_date = data.subscription_end_date,
        subscription_fee_desc = subscription_fee_desc,
        service_term = data.service_term,
        solutions = data.solutions,
        outlets_line = outlets_line,
        scope_items = scope_items_html,
        complementary = complementary_html,
        implementation_plan = implementation_plan_html,
        subscription_payments = sub_payments_html,
        subscription_total = subscription_total_str,
        implementation_payments = impl_payments_html,
        implementation_fee = data.implementation_fee,
        customer_officer_name = data.customer_officer_name,
        customer_officer_title = data.customer_officer_title,
    )
}

// ============================================================================
// PDF Generator - Proposal (HTML + Chrome headless)
// ============================================================================

/// Generate proposal PDF from markdown using HTML template + Chrome headless
pub fn generate_proposal_pdf(markdown: &str, output_path: &str) -> CmdResult<String> {
    // Step 1: Convert markdown to HTML
    let html_body = markdown_to_html(markdown);

    // Step 2: Extract title from markdown (first H1)
    let title = extract_title_from_markdown(markdown);

    // Step 3: Wrap in HTML template
    let full_html = wrap_in_proposal_template(&html_body, &title);

    // Step 4: Write HTML to temp file
    let temp_dir = std::env::temp_dir();
    let html_path = temp_dir.join(format!("proposal_{}.html", std::process::id()));
    let mut html_file = fs::File::create(&html_path)?;
    html_file.write_all(full_html.as_bytes())?;

    // Step 5: Convert HTML to PDF using Chrome headless
    let chrome_path = find_chrome_path()?;

    let status = Command::new(&chrome_path)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--no-pdf-header-footer",
            &format!("--print-to-pdf={}", output_path),
            &html_path.to_string_lossy(),
        ])
        .status()?;

    // Clean up temp file
    let _ = fs::remove_file(&html_path);

    if !status.success() {
        return Err(CommandError::Internal("Chrome PDF generation failed".to_string()));
    }

    Ok(output_path.to_string())
}

/// Convert markdown to HTML using pulldown-cmark
fn markdown_to_html(markdown: &str) -> String {
    // Filter out YAML frontmatter
    let content = strip_frontmatter(markdown);

    // Set up parser options for GFM (tables, etc.)
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&content, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

/// Strip YAML frontmatter from markdown
fn strip_frontmatter(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        return markdown.to_string();
    }

    // Find the closing ---
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            // Return everything after the frontmatter
            return lines[i + 1..].join("\n");
        }
    }

    markdown.to_string()
}

/// Extract title from markdown (first H1)
fn extract_title_from_markdown(markdown: &str) -> String {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return trimmed[2..].to_string();
        }
    }
    "Proposal".to_string()
}

/// Wrap HTML content in the proposal template
fn wrap_in_proposal_template(body: &str, title: &str) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <style>
    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }}

    body {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
      font-size: 10.5pt;
      line-height: 1.6;
      color: #1e293b;
      background: white;
    }}

    .document {{
      max-width: 210mm;
      padding: 1.2cm 1.5cm;
      margin: 0 auto;
      background: white;
    }}

    /* Header with logo */
    .header {{
      margin-bottom: 24px;
      padding-bottom: 16px;
      border-bottom: 3px solid #00A0E3;
    }}

    .logo svg {{
      height: 45px;
    }}

    /* First H1 - Customer/Title styling */
    h1:first-of-type {{
      font-size: 24pt;
      font-weight: 700;
      color: #1e293b;
      margin: 0 0 8px 0;
      padding-bottom: 0;
      border-bottom: none;
    }}

    h1 {{
      font-size: 18pt;
      font-weight: 700;
      color: #1e293b;
      margin: 28px 0 14px 0;
      padding-bottom: 6px;
      border-bottom: 2px solid #e2e8f0;
      page-break-after: avoid;
    }}

    h2 {{
      font-size: 14pt;
      font-weight: 600;
      color: #1e293b;
      margin: 22px 0 10px 0;
      border-bottom: 1px solid #e2e8f0;
      padding-bottom: 6px;
      page-break-after: avoid;
    }}

    h3 {{
      font-size: 12pt;
      font-weight: 600;
      color: #1e293b;
      margin: 18px 0 8px 0;
      page-break-after: avoid;
    }}

    h4 {{
      font-size: 11pt;
      font-weight: 600;
      color: #475569;
      margin: 14px 0 6px 0;
      page-break-after: avoid;
    }}

    /* Keep heading with next content */
    h1 + *, h2 + *, h3 + *, h4 + * {{
      page-break-before: avoid;
    }}

    p {{
      margin: 0 0 10px 0;
      orphans: 3;
      widows: 3;
    }}

    ul, ol {{
      margin: 0 0 12px 0;
      padding-left: 22px;
      page-break-inside: avoid;
    }}

    li {{
      margin-bottom: 4px;
      page-break-inside: avoid;
    }}

    /* Tables - matching proposal template */
    table {{
      width: 100%;
      border-collapse: collapse;
      margin: 14px 0;
      font-size: 9.5pt;
      page-break-inside: auto;
    }}

    td, th {{
      border: 1px solid #cbd5e1;
      padding: 8px 12px;
      text-align: left;
      vertical-align: top;
    }}

    th {{
      background-color: #1e293b;
      font-weight: 600;
      color: #ffffff;
    }}

    tr {{
      page-break-inside: avoid;
    }}

    thead {{
      display: table-header-group;
    }}

    /* Alternate row colors */
    tbody tr:nth-child(even) {{
      background-color: #f8fafc;
    }}

    /* Right-align numeric columns (2nd column onwards if they look like numbers) */
    td:nth-child(n+2):not(:last-child) {{
      text-align: right;
    }}

    /* Last column right-align for amounts */
    th:last-child, td:last-child {{
      text-align: right;
    }}

    /* First column left align */
    td:first-child, th:first-child {{
      text-align: left;
    }}

    /* Bold rows (rows with ** markdown) */
    tr:has(td strong:first-child) {{
      background-color: #f1f5f9 !important;
      font-weight: 600;
    }}

    strong {{
      font-weight: 600;
      color: #1e293b;
    }}

    /* Bold text in table body - blue to match accent color */
    td strong {{
      color: #0284c7;
    }}

    /* Horizontal rules */
    hr {{
      border: none;
      border-top: 1px solid #e2e8f0;
      margin: 20px 0;
    }}

    /* Code/YAML blocks - hide them */
    pre, code {{
      display: none;
    }}

    /* Blockquotes for callouts */
    blockquote {{
      margin: 14px 0;
      padding: 12px 16px;
      background-color: #f0f9ff;
      border-left: 4px solid #00A0E3;
      color: #0369a1;
      font-style: normal;
    }}

    blockquote p {{
      margin: 0;
    }}

    /* Emphasis text */
    em {{
      font-style: italic;
      color: #64748b;
    }}

    /* Footer */
    .footer {{
      margin-top: 36px;
      padding-top: 16px;
      border-top: 2px solid #e2e8f0;
      font-size: 8.5pt;
      color: #64748b;
      text-align: center;
    }}

    .footer p {{
      margin: 3px 0;
    }}

    .footer strong {{
      color: #475569;
    }}

    /* Page settings */
    @page {{
      size: A4;
      margin: 1.2cm;
    }}

    @media print {{
      .document {{
        padding: 0;
      }}

      /* Ensure tables don't break badly */
      table {{
        page-break-inside: auto;
      }}

      tr {{
        page-break-inside: avoid;
        page-break-after: auto;
      }}
    }}

    /* Special styling for pricing tables */
    table:has(th:contains("Annual")),
    table:has(th:contains("Monthly")),
    table:has(th:contains("Year")) {{
      font-size: 9pt;
    }}

    /* Highlight total rows */
    tr:has(td:first-child strong:contains("Total")),
    tr:has(td:first-child:contains("**Total")),
    tr:last-child:has(strong) {{
      background-color: #f1f5f9 !important;
      font-weight: 600;
    }}
  </style>
</head>
<body>
  <div class="document">
    <!-- Header -->
    <header class="header">
      <div class="logo">
        <img src="data:image/png;base64,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" alt="ThinkVAL" height="45">
      </div>
    </header>

    <!-- Content -->
    <main>
{body}
    </main>

    <!-- Footer -->
    <footer class="footer">
      <p><strong>ThinkVAL Pte. Ltd.</strong></p>
      <p>This document is confidential and intended solely for the addressee. Unauthorized distribution is prohibited.</p>
    </footer>
  </div>
</body>
</html>"##, title = title, body = body)
}

/// Find Chrome executable path
fn find_chrome_path() -> CmdResult<String> {
    // macOS paths
    let mac_paths = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];

    // Linux paths
    let linux_paths = [
        "/usr/bin/google-chrome",
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
    ];

    // Windows paths
    let windows_paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    for path in mac_paths.iter().chain(linux_paths.iter()).chain(windows_paths.iter()) {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    Err(CommandError::NotFound("Chrome not found. Please install Google Chrome.".to_string()))
}


/// Generate proposal PDF from markdown file
pub fn generate_proposal_from_file(
    file_path: &str,
    output_path: Option<&str>,
) -> CmdResult<String> {
    // Read markdown file
    let markdown = fs::read_to_string(file_path)?;

    // Determine output path
    let output = output_path
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let path = Path::new(file_path);
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let parent = path.parent().unwrap_or(Path::new("."));
            parent.join(format!("{}.pdf", stem)).to_string_lossy().to_string()
        });

    // Generate PDF
    generate_proposal_pdf(&markdown, &output)
}

// ============================================================================
// Public API
// ============================================================================

/// Generate order form PDF from markdown file
pub fn generate_order_form_from_file(
    file_path: &str,
    output_path: Option<&str>,
) -> CmdResult<String> {
    // Read markdown file
    let markdown = fs::read_to_string(file_path)?;

    // Parse data
    let data = parse_order_form_markdown(&markdown)?;

    // Determine output path
    let output = output_path
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let path = Path::new(file_path);
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let parent = path.parent().unwrap_or(Path::new("."));
            parent.join(format!("{}.pdf", stem)).to_string_lossy().to_string()
        });

    // Generate PDF
    generate_order_form_pdf(&data, &output)
}

/// Check if a file is an order form data file
#[allow(dead_code)]
pub fn is_order_form_file(file_path: &str) -> bool {
    let path = Path::new(file_path);
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    filename == "order-form-data.md" || filename.ends_with("-order-form.md")
}

/// Check if a file is a proposal data file
#[allow(dead_code)]
pub fn is_proposal_file(file_path: &str) -> bool {
    let path = Path::new(file_path);
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    filename == "proposal-data.md" || filename.ends_with("-proposal.md")
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Tauri command to generate order form PDF

pub async fn generate_order_form_pdf_cmd(file_path: String) -> CmdResult<String> {
    generate_order_form_from_file(&file_path, None)
}

/// Tauri command to generate proposal PDF

pub async fn generate_proposal_pdf_cmd(file_path: String) -> CmdResult<String> {
    generate_proposal_from_file(&file_path, None)
}

/// Tauri command to convert any HTML file to PDF using Chrome headless

pub async fn html_to_pdf_cmd(file_path: String) -> CmdResult<String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(CommandError::Internal(format!("File not found: {}", file_path)));
    }

    // Output PDF next to the HTML file with same name
    let output_path = path.with_extension("pdf");

    let chrome_path = find_chrome_path()?;

    let status = Command::new(&chrome_path)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--no-pdf-header-footer",
            &format!("--print-to-pdf={}", output_path.to_string_lossy()),
            &file_path,
        ])
        .status()?;

    if !status.success() {
        return Err(CommandError::Internal("Chrome PDF generation failed".to_string()));
    }

    Ok(output_path.to_string_lossy().to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_yaml_values() {
        let markdown = r#"
# Test

```yaml
customerName: Test Company
subscriptionFee: "10,000"
```
"#;
        let values = extract_yaml_values(markdown);
        assert_eq!(values.get("customerName"), Some(&"Test Company".to_string()));
        assert_eq!(values.get("subscriptionFee"), Some(&"10,000".to_string()));
    }

    #[test]
    fn test_extract_list_items() {
        let markdown = r#"
### Scope Items

- Item 1
- Item 2
- Item 3

### Other
"#;
        let items = extract_list_items(markdown, "Scope Items");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "Item 1");
    }
}
