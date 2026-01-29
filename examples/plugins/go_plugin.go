// Example Go Plugin for Carapace
//
// This is a sample Go plugin that can be compiled to WASM
// using TinyGo and loaded by the carapace gateway.
//
// Prerequisites:
//   - Install TinyGo: https://tinygo.org/getting-started/install/
//
// To compile:
//   tinygo build -target wasi -o go_plugin.wasm main.go
//
// Place the compiled WASM file in ~/.carapace/plugins/go/

package main

import (
	"encoding/json"
	"fmt"
	"strconv"
)

// Required plugin metadata
var PluginName = "go-plugin"
var PluginVersion = "0.1.0"
var PluginDescription = "A Go plugin example demonstrating WASM plugin development"

// greetInput represents input for the greet tool
type greetInput struct {
	Name    string `json:"name"`
	Prefix  string `json:"prefix,omitempty"`
}

// calculateInput represents input for the calculate tool
type calculateInput struct {
	A        float64 `json:"a"`
	B        float64 `json:"b"`
	Operation string `json:"operation"`
}

// Tool: greet - Generate a personalized greeting
func greet(name string, prefix string) map[string]interface{} {
	if prefix == "" {
		prefix = "Hello"
	}
	greeting := fmt.Sprintf("%s, %s!", prefix, name)
	return map[string]interface{}{
		"greeting":  greeting,
		"length":    len(greeting),
		"uppercase": greeting,
	}
}

// Tool: calculate - Perform a simple calculation
func calculate(a, b float64, operation string) map[string]interface{} {
	var result float64

	switch operation {
	case "add":
		result = a + b
	case "subtract":
		result = a - b
	case "multiply":
		result = a * b
	case "divide":
		if b != 0 {
			result = a / b
		}
	}

	return map[string]interface{}{
		"operation": operation,
		"a":         a,
		"b":         b,
		"result":    result,
	}
}

// Tool: echo - Echo back a message with optional repetition
func echo(message string, repeat int) map[string]interface{} {
	if repeat < 1 {
		repeat = 1
	}
	repeated := ""
	for i := 0; i < repeat; i++ {
		repeated += message + " "
	}
	repeated = repeated[:len(repeated)-1] // Remove trailing space

	return map[string]interface{}{
		"original":  message,
		"repeated":  repeated,
		"repeatCount": repeat,
	}
}

// Tool: getInfo - Return information about this plugin
func getInfo() map[string]interface{} {
	return map[string]interface{}{
		"name":        PluginName,
		"version":     PluginVersion,
		"description": PluginDescription,
		"tools": []map[string]interface{}{
			{
				"name":        "greet",
				"description": "Generate a personalized greeting",
				"params": map[string]interface{}{
					"name":   map[string]string{"type": "string", "description": "Name to greet"},
					"prefix": map[string]string{"type": "string", "description": "Greeting prefix (optional)"},
				},
			},
			{
				"name":        "calculate",
				"description": "Perform a simple calculation",
				"params": map[string]interface{}{
					"a":        map[string]string{"type": "number", "description": "First number"},
					"b":        map[string]string{"type": "number", "description": "Second number"},
					"operation": map[string]string{"type": "string", "description": "Operation: add, subtract, multiply, divide"},
				},
			},
			{
				"name":        "echo",
				"description": "Echo back a message with optional repetition",
				"params": map[string]interface{}{
					"message": map[string]string{"type": "string", "description": "Message to echo"},
					"repeat":  map[string]string{"type": "number", "description": "Number of times to repeat (default: 1)"},
				},
			},
			{
				"name":        "getInfo",
				"description": "Get plugin information",
				"params":      map[string]interface{}{},
			},
		},
	}
}

// Tool: transformText - Apply text transformations
func transformText(text string, options string) map[string]interface{} {
	var opts map[string]interface{}
	json.Unmarshal([]byte(options), &opts)

	uppercase := false
	lowercase := false
	reverse := false
	trim := true
	capitalize := false

	if v, ok := opts["uppercase"].(bool); ok {
		uppercase = v
	}
	if v, ok := opts["lowercase"].(bool); ok {
		lowercase = v
	}
	if v, ok := opts["reverse"].(bool); ok {
		reverse = v
	}
	if v, ok := opts["trim"].(bool); ok {
		trim = v
	}
	if v, ok := opts["capitalize"].(bool); ok {
		capitalize = v
	}

	result := text
	if trim {
		result = removeSpaces(result)
	}
	if uppercase {
		result = toUpper(result)
	} else if lowercase {
		result = toLower(result)
	} else if capitalize {
		result = capitalizeFirst(result)
	}
	if reverse {
		result = reverseString(result)
	}

	return map[string]interface{}{
		"original":   text,
		"transformed": result,
		"options":    opts,
	}
}

// Helper functions for string manipulation
func removeSpaces(s string) string {
	result := ""
	for _, c := range s {
		if c != ' ' && c != '\t' && c != '\n' && c != '\r' {
			result += string(c)
		}
	}
	return result
}

func toUpper(s string) string {
	result := ""
	for _, c := range s {
		if c >= 'a' && c <= 'z' {
			result += string(c - 32)
		} else {
			result += string(c)
		}
	}
	return result
}

func toLower(s string) string {
	result := ""
	for _, c := range s {
		if c >= 'A' && c <= 'Z' {
			result += string(c + 32)
		} else {
			result += string(c)
		}
	}
	return result
}

func capitalizeFirst(s string) string {
	if len(s) == 0 {
		return s
	}
	return toUpper(string(s[0])) + s[1:]
}

func reverseString(s string) string {
	result := ""
	for i := len(s) - 1; i >= 0; i-- {
		result += string(s[i])
	}
	return result
}

// Main entry point - handle tool calls from carapace
//export HandleTool
func HandleTool(toolName string, argsJSON string) string {
	var args map[string]interface{}
	if argsJSON != "" {
		json.Unmarshal([]byte(argsJSON), &args)
	}

	switch toolName {
	case "greet":
		name := ""
		prefix := ""
		if n, ok := args["name"].(string); ok {
			name = n
		}
		if p, ok := args["prefix"].(string); ok {
			prefix = p
		}
		result, _ := json.Marshal(greet(name, prefix))
		return string(result)

	case "calculate":
		var a, b float64
		operation := "add"
		if val, ok := args["a"].(float64); ok {
			a = val
		}
		if val, ok := args["b"].(float64); ok {
			b = val
		}
		if val, ok := args["operation"].(string); ok {
			operation = val
		}
		result, _ := json.Marshal(calculate(a, b, operation))
		return string(result)

	case "echo":
		message := ""
		repeat := 1
		if m, ok := args["message"].(string); ok {
			message = m
		}
		if r, ok := args["repeat"].(float64); ok {
			repeat = int(r)
		}
		result, _ := json.Marshal(echo(message, repeat))
		return string(result)

	case "getInfo":
		result, _ := json.Marshal(getInfo())
		return string(result)

	case "transformText":
		text := ""
		options := "{}"
		if t, ok := args["text"].(string); ok {
			text = t
		}
		if o, ok := args["options"].(string); ok {
			options = o
		} else if o, ok := args["options"].(map[string]interface{}); ok {
			options, _ = json.Marshal(o)
		}
		result, _ := json.Marshal(transformText(text, options))
		return string(result)

	default:
		return `{"error": "Unknown tool: ` + toolName + `"}`
	}
}

// Get plugin info - exported for discovery
//export GetInfo
func GetInfo() string {
	result, _ := json.Marshal(getInfo())
	return string(result)
}

// Init - called when plugin is loaded
//export Init
func Init() int {
	fmt.Printf("%s v%s initialized\n", PluginName, PluginVersion)
	return 0
}

// Shutdown - called when plugin is unloaded
//export Shutdown
func Shutdown() int {
	fmt.Printf("%s shutdown\n", PluginName)
	return 0
}

// Required for TinyGo WASI
func main() {
	// main() is required for TinyGo but not used directly
	// The actual entry point is via exported functions
}
