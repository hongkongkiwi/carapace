//! JSON Schema Generation
//!
//! Generates JSON Schema for carapace configuration validation.
//! Based on draft-07 of the JSON Schema specification.

use serde_json::json;
use serde_json::Value;

/// Generate the complete JSON schema for carapace configuration
pub fn generate_config_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Carapace Configuration",
        "description": "Configuration schema for the Carapace messaging gateway",
        "type": "object",
        "properties": {
            "meta": generate_meta_schema(),
            "env": generate_env_schema(),
            "wizard": generate_wizard_schema(),
            "diagnostics": generate_diagnostics_schema(),
            "hooks": generate_hooks_schema(),
            "gateway": generate_gateway_schema(),
            "ui": generate_ui_schema(),
            "openai": generate_openai_schema(),
            "control": generate_control_schema(),
            "channels": generate_channels_schema(),
            "plugins": generate_plugins_schema(),
            "credentials": generate_credentials_schema(),
            "autoreply": generate_autoreply_schema(),
            "polls": generate_polls_schema(),
            "flows": generate_flows_schema(),
            "nodes": generate_nodes_schema(),
            "security": generate_security_schema(),
            "usage": generate_usage_schema(),
            "media": generate_media_schema(),
            "tts": generate_tts_schema(),
            "tracing": generate_tracing_schema(),
            "metrics": generate_metrics_schema(),
            "migrations": generate_migrations_schema(),
        },
        "additionalProperties": false
    })
}

fn generate_meta_schema() -> Value {
    json!({
        "type": "object",
        "description": "Gateway metadata",
        "properties": {
            "name": {
                "type": "string",
                "description": "Human-readable gateway name"
            },
            "version": {
                "type": "string",
                "description": "Gateway version"
            },
            "description": {
                "type": "string",
                "description": "Gateway description"
            }
        }
    })
}

fn generate_env_schema() -> Value {
    json!({
        "type": "object",
        "description": "Environment-specific configuration overrides",
        "additionalProperties": true
    })
}

fn generate_wizard_schema() -> Value {
    json!({
        "type": "object",
        "description": "Interactive setup wizard configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable the setup wizard",
                "default": true
            },
            "port": {
                "type": "integer",
                "description": "Wizard web server port",
                "default": 8080
            }
        }
    })
}

fn generate_diagnostics_schema() -> Value {
    json!({
        "type": "object",
        "description": "Diagnostics and debugging configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable diagnostics endpoints"
            },
            "endpoint": {
                "type": "string",
                "description": "Diagnostics endpoint path"
            }
        }
    })
}

fn generate_hooks_schema() -> Value {
    json!({
        "type": "object",
        "description": "Webhook hooks configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable hooks endpoints"
            },
            "path": {
                "type": "string",
                "description": "Hooks endpoint path"
            },
            "maxBodyBytes": {
                "type": "integer",
                "description": "Maximum request body size in bytes",
                "default": 262144
            },
            "token": {
                "type": "string",
                "description": "Hooks authentication token (use env var to avoid exposure)"
            }
        }
    })
}

fn generate_gateway_schema() -> Value {
    json!({
        "type": "object",
        "description": "Main gateway configuration",
        "properties": {
            "name": {
                "type": "string",
                "description": "Gateway instance name"
            },
            "port": {
                "type": "integer",
                "description": "Gateway server port",
                "default": 8080
            },
            "bind": {
                "type": "string",
                "description": "Bind address (host:port format)"
            },
            "stateDir": {
                "type": "string",
                "description": "Directory for state files"
            },
            "dev": {
                "type": "boolean",
                "description": "Development mode (localhost-only, no auth)"
            },
            "tls": {
                "type": "object",
                "description": "TLS configuration",
                "properties": {
                    "enabled": {
                        "type": "boolean"
                    },
                    "cert": {
                        "type": "string",
                        "description": "Path to TLS certificate"
                    },
                    "key": {
                        "type": "string",
                        "description": "Path to TLS private key"
                    }
                }
            },
            "password": {
                "type": "string",
                "description": "Gateway authentication password"
            },
            "token": {
                "type": "string",
                "description": "Gateway authentication token"
            },
            "channels": {
                "type": "array",
                "description": "List of channel configurations",
                "items": {
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["console", "discord", "telegram", "whatsapp", "slack", "teams", "signal", "imessage", "google_chat", "line", "matrix", "skype", "webchat", "zalo", "webhook", "voice"]
                        },
                        "enabled": {
                            "type": "boolean"
                        }
                    },
                    "required": ["type"]
                }
            }
        }
    })
}

fn generate_ui_schema() -> Value {
    json!({
        "type": "object",
        "description": "Control UI configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable the control UI"
            },
            "basePath": {
                "type": "string",
                "description": "UI base path"
            },
            "distPath": {
                "type": "string",
                "description": "Path to UI distribution files"
            }
        }
    })
}

fn generate_openai_schema() -> Value {
    json!({
        "type": "object",
        "description": "OpenAI-compatible API configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable OpenAI-compatible endpoints"
            },
            "apiKeys": {
                "type": "array",
                "description": "Allowed API keys",
                "items": {
                    "type": "string"
                }
            }
        }
    })
}

fn generate_control_schema() -> Value {
    json!({
        "type": "object",
        "description": "Control endpoints configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable control endpoints"
            }
        }
    })
}

fn generate_channels_schema() -> Value {
    json!({
        "type": "object",
        "description": "Channel-specific configurations",
        "additionalProperties": true
    })
}

fn generate_plugins_schema() -> Value {
    json!({
        "type": "object",
        "description": "Plugin configuration",
        "properties": {
            "dir": {
                "type": "string",
                "description": "Plugin directory path"
            },
            "enabled": {
                "type": "boolean",
                "description": "Enable plugins"
            },
            "hotReload": {
                "type": "object",
                "description": "Hot reload configuration",
                "properties": {
                    "enabled": {
                        "type": "boolean"
                    },
                    "debounceMs": {
                        "type": "integer"
                    }
                }
            }
        }
    })
}

fn generate_credentials_schema() -> Value {
    json!({
        "type": "object",
        "description": "Credentials configuration",
        "properties": {
            "keychain": {
                "type": "object",
                "description": "Keychain/backing store configuration",
                "properties": {
                    "enabled": {
                        "type": "boolean"
                    }
                }
            }
        }
    })
}

fn generate_autoreply_schema() -> Value {
    json!({
        "type": "object",
        "description": "Auto-reply configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable auto-reply functionality"
            }
        }
    })
}

fn generate_polls_schema() -> Value {
    json!({
        "type": "object",
        "description": "Polls configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable polls"
            }
        }
    })
}

fn generate_flows_schema() -> Value {
    json!({
        "type": "object",
        "description": "Flows configuration",
        "properties": {
            "enabled": {
                "type": "boolean",
                "description": "Enable flows"
            }
        }
    })
}

fn generate_nodes_schema() -> Value {
    json!({
        "type": "object",
        "description": "Nodes configuration",
        "properties": {
            "enabled": {
                "type": "boolean"
            }
        }
    })
}

fn generate_security_schema() -> Value {
    json!({
        "type": "object",
        "description": "Security configuration",
        "properties": {
            "allowedHosts": {
                "type": "array",
                "description": "List of allowed host headers",
                "items": {
                    "type": "string"
                }
            },
            "cors": {
                "type": "object",
                "description": "CORS configuration",
                "properties": {
                    "enabled": {
                        "type": "boolean"
                    },
                    "origins": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    }
                }
            },
            "rateLimit": {
                "type": "object",
                "description": "Rate limiting configuration",
                "properties": {
                    "enabled": {
                        "type": "boolean"
                    },
                    "requestsPerMinute": {
                        "type": "integer"
                    }
                }
            }
        }
    })
}

fn generate_usage_schema() -> Value {
    json!({
        "type": "object",
        "description": "Usage tracking configuration",
        "properties": {
            "enabled": {
                "type": "boolean"
            },
            "storage": {
                "type": "string",
                "description": "Storage backend (sqlite/postgres)"
            }
        }
    })
}

fn generate_media_schema() -> Value {
    json!({
        "type": "object",
        "description": "Media processing configuration",
        "properties": {
            "enabled": {
                "type": "boolean"
            },
            "dir": {
                "type": "string",
                "description": "Media storage directory"
            },
            "maxSize": {
                "type": "integer",
                "description": "Maximum media file size in bytes"
            }
        }
    })
}

fn generate_tts_schema() -> Value {
    json!({
        "type": "object",
        "description": "Text-to-speech configuration",
        "properties": {
            "enabled": {
                "type": "boolean"
            },
            "defaultEngine": {
                "type": "string",
                "description": "Default TTS engine"
            }
        }
    })
}

fn generate_tracing_schema() -> Value {
    json!({
        "type": "object",
        "description": "Tracing configuration",
        "properties": {
            "enabled": {
                "type": "boolean"
            },
            "serviceName": {
                "type": "string"
            },
            "logLevel": {
                "type": "string",
                "enum": ["debug", "info", "warn", "error"]
            },
            "exporter": {
                "type": "string",
                "enum": ["json", "text", "opentelemetry"]
            }
        }
    })
}

fn generate_metrics_schema() -> Value {
    json!({
        "type": "object",
        "description": "Metrics configuration",
        "properties": {
            "enabled": {
                "type": "boolean"
            },
            "path": {
                "type": "string",
                "description": "Metrics endpoint path"
            },
            "port": {
                "type": "integer",
                "description": "Metrics server port"
            }
        }
    })
}

fn generate_migrations_schema() -> Value {
    json!({
        "type": "object",
        "description": "Database migrations configuration",
        "properties": {
            "enabled": {
                "type": "boolean"
            },
            "dir": {
                "type": "string",
                "description": "Migrations directory"
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config_schema() {
        let schema = generate_config_schema();

        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(schema["title"], "Carapace Configuration");
        assert!(schema["properties"].is_object());
        assert!(schema["properties"]["meta"].is_object());
        assert!(schema["properties"]["gateway"].is_object());
        assert!(schema["properties"]["hooks"].is_object());
        assert!(schema["properties"]["channels"].is_object());
    }

    #[test]
    fn test_schema_is_valid_json() {
        let schema = generate_config_schema();
        let json_str = serde_json::to_string(&schema);
        assert!(json_str.is_ok());
    }

    #[test]
    fn test_gateway_schema_has_required_fields() {
        let schema = generate_config_schema();
        let gateway = &schema["properties"]["gateway"];

        assert!(gateway["properties"]["port"].is_object());
        assert!(gateway["properties"]["bind"].is_object());
        assert!(gateway["properties"]["tls"].is_object());
    }

    #[test]
    fn test_channels_schema_has_enum() {
        let schema = generate_config_schema();
        let channels = &schema["properties"]["channels"]["items"]["properties"]["type"];

        let enum_values = channels["enum"].as_array().unwrap();
        assert!(enum_values.contains(&json!("console")));
        assert!(enum_values.contains(&json!("telegram")));
        assert!(enum_values.contains(&json!("discord")));
    }
}
