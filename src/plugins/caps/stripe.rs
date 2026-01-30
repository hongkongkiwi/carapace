//! Stripe Tool Plugin
//!
//! Native implementation of Stripe API operations for carapace.
//! Supports payments, customers, and products.
//!
//! Security: API key retrieved via credential_get() - never hardcoded.

use crate::plugins::bindings::{
    BindingError, ToolContext, ToolDefinition, ToolPluginInstance, ToolResult,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Stripe tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConfig {
    /// Secret API key
    #[serde(skip)]
    pub api_key: Option<String>,

    /// API version
    #[serde(default = "default_api_version")]
    pub api_version: String,

    /// Default currency
    #[serde(default = "default_currency")]
    pub default_currency: String,
}

fn default_api_version() -> String {
    "2023-10-16".to_string()
}
fn default_currency() -> String {
    "usd".to_string()
}

impl Default for StripeConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_version: default_api_version(),
            default_currency: default_currency(),
        }
    }
}

/// Stripe API client
#[derive(Debug, Clone)]
pub struct StripeClient {
    config: StripeConfig,
    http_client: reqwest::blocking::Client,
}

impl StripeClient {
    pub fn new(config: StripeConfig) -> Result<Self, BindingError> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| BindingError::CallError(e.to_string()))?;
        Ok(Self {
            config,
            http_client,
        })
    }

    pub fn with_key(mut self, key: String) -> Self {
        self.config.api_key = Some(key);
        self
    }

    fn auth_headers(&self) -> Result<Vec<(String, String)>, BindingError> {
        let key =
            self.config.api_key.as_ref().ok_or_else(|| {
                BindingError::CallError("Stripe API key not configured".to_string())
            })?;
        Ok(vec![(
            "Authorization".to_string(),
            format!("Bearer {}", key),
        )])
    }

    fn request(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, BindingError> {
        let url = format!("https://api.stripe.com/v1{}", endpoint);
        let mut request = self.http_client.request(method, &url);
        for (k, v) in self.auth_headers()? {
            request = request.header(k, v);
        }
        request = request.header("Stripe-Version", &self.config.api_version);
        request = request.header("Content-Type", "application/x-www-form-urlencoded");
        if let Some(b) = body {
            request = request.body(Self::form_encode(&b));
        }
        let resp = request
            .send()
            .map_err(|e| BindingError::CallError(format!("Request failed: {}", e)))?;
        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_else(|_| "Unknown".to_string());
            return Err(BindingError::CallError(format!(
                "Stripe API error: {}",
                text
            )));
        }
        resp.json()
            .map_err(|e| BindingError::CallError(format!("Parse error: {}", e)))
    }

    fn form_encode(data: &serde_json::Value) -> String {
        match data {
            serde_json::Value::Object(map) => map
                .iter()
                .filter_map(|(k, v)| {
                    Some(format!("{}={}", k, v.as_str().unwrap_or("")))
                        .filter(|s| !s.contains("null"))
                })
                .collect::<Vec<_>>()
                .join("&"),
            _ => String::new(),
        }
    }

    /// Create customer
    pub fn create_customer(
        &self,
        email: &str,
        name: Option<&str>,
    ) -> Result<serde_json::Value, BindingError> {
        let body = json!({ "email": email, "name": name.unwrap_or("") });
        self.request(reqwest::Method::POST, "/customers", Some(body))
    }

    /// Create payment intent
    pub fn create_payment_intent(
        &self,
        amount: i64,
        currency: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<serde_json::Value, BindingError> {
        let mut body = json!({ "amount": amount, "currency": currency.unwrap_or(&self.config.default_currency) });
        if let Some(cid) = customer_id {
            body["customer"] = serde_json::Value::String(cid.to_string());
        }
        self.request(reqwest::Method::POST, "/payment_intents", Some(body))
    }

    /// Retrieve customer
    pub fn get_customer(&self, customer_id: &str) -> Result<serde_json::Value, BindingError> {
        self.request(
            reqwest::Method::GET,
            &format!("/customers/{}", customer_id),
            None,
        )
    }

    /// List payments
    pub fn list_payments(&self, limit: Option<i32>) -> Result<serde_json::Value, BindingError> {
        let endpoint = format!("/payment_intents?limit={}", limit.unwrap_or(10));
        self.request(reqwest::Method::GET, &endpoint, None)
    }
}

/// Stripe tool plugin
#[derive(Debug, Clone)]
pub struct StripeTool {
    client: Option<StripeClient>,
}

impl StripeTool {
    pub fn new() -> Self {
        Self { client: None }
    }
    pub fn initialize(&mut self, config: StripeConfig) -> Result<(), BindingError> {
        let client = StripeClient::new(config)?;
        self.client = Some(client);
        Ok(())
    }
}

impl Default for StripeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolPluginInstance for StripeTool {
    fn get_definitions(&self) -> Result<Vec<ToolDefinition>, BindingError> {
        Ok(vec![
            ToolDefinition {
                name: "stripe_create_customer".to_string(),
                description: "Create a Stripe customer.".to_string(),
                input_schema: CustomerInput::schema().to_string(),
            },
            ToolDefinition {
                name: "stripe_create_payment".to_string(),
                description: "Create a payment intent.".to_string(),
                input_schema: PaymentInput::schema().to_string(),
            },
            ToolDefinition {
                name: "stripe_get_customer".to_string(),
                description: "Get customer details.".to_string(),
                input_schema: GetCustomerInput::schema().to_string(),
            },
            ToolDefinition {
                name: "stripe_list_payments".to_string(),
                description: "List recent payments.".to_string(),
                input_schema: ListInput::schema().to_string(),
            },
        ])
    }

    fn invoke(
        &self,
        name: &str,
        params: &str,
        _ctx: ToolContext,
    ) -> Result<ToolResult, BindingError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| BindingError::CallError("Stripe tool not initialized".to_string()))?;
        match name {
            "stripe_create_customer" => {
                let input: CustomerInput = serde_json::from_str(params)?;
                let result = client.create_customer(&input.email, input.name.as_deref())?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "stripe_create_payment" => {
                let input: PaymentInput = serde_json::from_str(params)?;
                let result = client.create_payment_intent(
                    input.amount,
                    input.currency.as_deref(),
                    input.customer_id.as_deref(),
                )?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "stripe_get_customer" => {
                let input: GetCustomerInput = serde_json::from_str(params)?;
                let result = client.get_customer(&input.customer_id)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            "stripe_list_payments" => {
                let result = client.list_payments(None)?;
                Ok(ToolResult {
                    success: true,
                    result: Some(serde_json::to_string(&result)?),
                    error: None,
                })
            }
            _ => Err(BindingError::CallError(format!("Unknown tool: {}", name))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerInput {
    pub email: String,
    pub name: Option<String>,
}

impl CustomerInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"email": {"type": "string"}, "name": {"type": "string"}}, "required": ["email"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInput {
    pub amount: i64,
    pub currency: Option<String>,
    pub customer_id: Option<String>,
}

impl PaymentInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"amount": {"type": "integer"}, "currency": {"type": "string"}, "customer_id": {"type": "string"}}, "required": ["amount"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCustomerInput {
    pub customer_id: String,
}

impl GetCustomerInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {"customer_id": {"type": "string"}}, "required": ["customer_id"]})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInput;

impl ListInput {
    fn schema() -> serde_json::Value {
        json!({"type": "object", "properties": {}})
    }
}
