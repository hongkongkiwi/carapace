//! Carapace Web UI
//!
//! Leptos-based web dashboard for managing the carapace gateway.

use leptos::*;
use leptos_router::components::{Router, Routes, Route, A};

/// Root layout component
#[component]
pub fn AppRoot() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-slate-900 text-slate-100">
            <NavBar />
            <main class="container mx-auto px-4 py-8">
                <AppRouter />
            </main>
        </div>
    }
}

/// Main router component
#[component]
fn AppRouter() -> impl IntoView {
    view! {
        <Router>
            <Route path="/" view=DashboardPage />
            <Route path="/channels" view=ChannelsPage />
            <Route path="/channels/:id" view=ChannelDetailPage />
            <Route path="/agents" view=AgentsPage />
            <Route path="/logs" view=LogsPage />
            <Route path="/settings" view=SettingsPage />
        </Router>
    }
}

/// Navigation bar component
#[component]
fn NavBar() -> impl IntoView {
    view! {
        <nav class="bg-slate-800/50 border-b border-slate-700 backdrop-blur-sm sticky top-0 z-50">
            <div class="container mx-auto px-4">
                <div class="flex items-center justify-between h-16">
                    <div class="flex items-center space-x-8">
                        <div class="flex items-center space-x-3">
                            <div class="w-8 h-8 bg-indigo-500 rounded-lg flex items-center justify-center">
                                <span class="text-white font-bold text-sm">C</span>
                            </div>
                            <h1 class="text-xl font-bold bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
                                "Carapace"
                            </h1>
                        </div>
                        <div class="hidden md:flex items-center space-x-1">
                            <A href="/" class="px-4 py-2 text-sm font-medium text-slate-300 hover:text-white hover:bg-slate-700/50 rounded-lg transition-all">
                                "Dashboard"
                            </A>
                            <A href="/channels" class="px-4 py-2 text-sm font-medium text-slate-300 hover:text-white hover:bg-slate-700/50 rounded-lg transition-all">
                                "Channels"
                            </A>
                            <A href="/agents" class="px-4 py-2 text-sm font-medium text-slate-300 hover:text-white hover:bg-slate-700/50 rounded-lg transition-all">
                                "Agents"
                            </A>
                            <A href="/logs" class="px-4 py-2 text-sm font-medium text-slate-300 hover:text-white hover:bg-slate-700/50 rounded-lg transition-all">
                                "Logs"
                            </A>
                            <A href="/settings" class="px-4 py-2 text-sm font-medium text-slate-300 hover:text-white hover:bg-slate-700/50 rounded-lg transition-all">
                                "Settings"
                            </A>
                        </div>
                    </div>
                    <div class="flex items-center space-x-4">
                        <div class="flex items-center space-x-2 px-3 py-1.5 bg-slate-700/50 rounded-full">
                            <span class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
                            <span class="text-xs font-medium text-slate-300">"Online"</span>
                        </div>
                    </div>
                </div>
            </div>
        </nav>
    }
}

/// Dashboard page component
#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <div class="space-y-8">
            <div class="flex items-center justify-between">
                <div>
                    <h2 class="text-3xl font-bold text-white">"Dashboard"</h2>
                    <p class="text-slate-400 mt-1">"Welcome back! Here's what's happening with your gateway."</p>
                </div>
                <div class="flex items-center space-x-3">
                    <button class="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-slate-300 font-medium rounded-lg transition-colors flex items-center space-x-2">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                        </svg>
                        <span>"Refresh"</span>
                    </button>
                </div>
            </div>

            {/* Stats Cards */}
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="w-12 h-12 bg-blue-500 rounded-xl flex items-center justify-center">
                            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
                            </svg>
                        </div>
                    </div>
                    <h3 class="text-slate-400 text-sm font-medium">"Channels"</h3>
                    <p class="text-2xl font-bold text-white mt-1">"6"</p>
                    <p class="text-xs text-slate-500 mt-2">"+2 this month"</p>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="w-12 h-12 bg-green-500 rounded-xl flex items-center justify-center">
                            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z"></path>
                            </svg>
                        </div>
                    </div>
                    <h3 class="text-slate-400 text-sm font-medium">"Messages Today"</h3>
                    <p class="text-2xl font-bold text-white mt-1">"1,234"</p>
                    <p class="text-xs text-slate-500 mt-2">"+12% vs yesterday"</p>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="w-12 h-12 bg-purple-500 rounded-xl flex items-center justify-center">
                            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
                            </svg>
                        </div>
                    </div>
                    <h3 class="text-slate-400 text-sm font-medium">"Active Agents"</h3>
                    <p class="text-2xl font-bold text-white mt-1">"3"</p>
                    <p class="text-xs text-slate-500 mt-2">"All running"</p>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="w-12 h-12 bg-indigo-500 rounded-xl flex items-center justify-center">
                            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                            </svg>
                        </div>
                    </div>
                    <h3 class="text-slate-400 text-sm font-medium">"Uptime"</h3>
                    <p class="text-2xl font-bold text-white mt-1">"99.9%"</p>
                    <p class="text-xs text-slate-500 mt-2">"Last 30 days"</p>
                </div>
            </div>

            {/* Activity Feed & Quick Actions */}
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                {/* Activity Feed */}
                <div class="lg:col-span-2 bg-slate-800/30 border border-slate-700/50 rounded-xl p-6">
                    <h3 class="text-lg font-semibold text-white mb-4">"Recent Activity"</h3>
                    <div class="space-y-4">
                        <div class="flex items-start space-x-4 p-4 bg-slate-700/20 rounded-lg hover:bg-slate-700/30 transition-colors">
                            <div class="w-10 h-10 bg-green-500 rounded-full flex items-center justify-center flex-shrink-0">
                                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
                                </svg>
                            </div>
                            <div class="flex-1 min-w-0">
                                <p class="text-sm font-medium text-white">"Message received from Telegram"</p>
                                <p class="text-sm text-slate-400 mt-0.5 truncate">"User @johndoe sent a message to the general channel"</p>
                            </div>
                            <span class="text-xs text-slate-500 whitespace-nowrap">"2 minutes ago"</span>
                        </div>

                        <div class="flex items-start space-x-4 p-4 bg-slate-700/20 rounded-lg hover:bg-slate-700/30 transition-colors">
                            <div class="w-10 h-10 bg-blue-500 rounded-full flex items-center justify-center flex-shrink-0">
                                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z"></path>
                                </svg>
                            </div>
                            <div class="flex-1 min-w-0">
                                <p class="text-sm font-medium text-white">"Discord notification sent"</p>
                                <p class="text-sm text-slate-400 mt-0.5 truncate">"Auto-reply triggered for user in #general"</p>
                            </div>
                            <span class="text-xs text-slate-500 whitespace-nowrap">"5 minutes ago"</span>
                        </div>

                        <div class="flex items-start space-x-4 p-4 bg-slate-700/20 rounded-lg hover:bg-slate-700/30 transition-colors">
                            <div class="w-10 h-10 bg-purple-500 rounded-full flex items-center justify-center flex-shrink-0">
                                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
                                </svg>
                            </div>
                            <div class="flex-1 min-w-0">
                                <p class="text-sm font-medium text-white">"Agent executed successfully"</p>
                                <p class="text-sm text-slate-400 mt-0.5 truncate">"Summary agent processed 15 messages"</p>
                            </div>
                            <span class="text-xs text-slate-500 whitespace-nowrap">"1 hour ago"</span>
                        </div>
                    </div>
                </div>

                {/* Quick Actions */}
                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6">
                    <h3 class="text-lg font-semibold text-white mb-4">"Quick Actions"</h3>
                    <div class="space-y-3">
                        <a href="/channels" class="flex items-center space-x-3 p-3 bg-slate-700/30 border border-slate-600/50 rounded-lg hover:border-slate-500 hover:bg-slate-700/50 transition-all group">
                            <div class="w-10 h-10 bg-slate-600/50 rounded-lg flex items-center justify-center group-hover:bg-indigo-500/20 group-hover:text-indigo-400 transition-colors">
                                <svg class="w-5 h-5 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                                </svg>
                            </div>
                            <div class="flex-1">
                                <p class="text-sm font-medium text-white group-hover:text-indigo-300 transition-colors">"Add Channel"</p>
                                <p class="text-xs text-slate-400">"Connect a new messaging platform"</p>
                            </div>
                        </a>

                        <a href="/agents" class="flex items-center space-x-3 p-3 bg-slate-700/30 border border-slate-600/50 rounded-lg hover:border-slate-500 hover:bg-slate-700/50 transition-all group">
                            <div class="w-10 h-10 bg-slate-600/50 rounded-lg flex items-center justify-center group-hover:bg-indigo-500/20 group-hover:text-indigo-400 transition-colors">
                                <svg class="w-5 h-5 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                                </svg>
                            </div>
                            <div class="flex-1">
                                <p class="text-sm font-medium text-white group-hover:text-indigo-300 transition-colors">"Create Agent"</p>
                                <p class="text-xs text-slate-400">"Build a new AI agent"</p>
                            </div>
                        </a>

                        <a href="/logs" class="flex items-center space-x-3 p-3 bg-slate-700/30 border border-slate-600/50 rounded-lg hover:border-slate-500 hover:bg-slate-700/50 transition-all group">
                            <div class="w-10 h-10 bg-slate-600/50 rounded-lg flex items-center justify-center group-hover:bg-indigo-500/20 group-hover:text-indigo-400 transition-colors">
                                <svg class="w-5 h-5 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                                </svg>
                            </div>
                            <div class="flex-1">
                                <p class="text-sm font-medium text-white group-hover:text-indigo-300 transition-colors">"View Logs"</p>
                                <p class="text-xs text-slate-400">"Check recent system logs"</p>
                            </div>
                        </a>

                        <a href="/settings" class="flex items-center space-x-3 p-3 bg-slate-700/30 border border-slate-600/50 rounded-lg hover:border-slate-500 hover:bg-slate-700/50 transition-all group">
                            <div class="w-10 h-10 bg-slate-600/50 rounded-lg flex items-center justify-center group-hover:bg-indigo-500/20 group-hover:text-indigo-400 transition-colors">
                                <svg class="w-5 h-5 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                                </svg>
                            </div>
                            <div class="flex-1">
                                <p class="text-sm font-medium text-white group-hover:text-indigo-300 transition-colors">"Settings"</p>
                                <p class="text-xs text-slate-400">"Configure gateway settings"</p>
                            </div>
                        </a>
                    </div>
                </div>
            </div>

            {/* Channel Status */}
            <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6">
                <h3 class="text-lg font-semibold text-white mb-4">"Channel Status"</h3>
                <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
                    <div class="p-4 border rounded-lg bg-green-500/10 border-green-500/20">
                        <div class="flex items-center space-x-2 mb-2">
                            <span class="w-2 h-2 rounded-full bg-green-500"></span>
                            <span class="text-xs font-medium text-green-400">"Connected"</span>
                        </div>
                        <p class="text-sm font-medium text-white">"Telegram"</p>
                    </div>
                    <div class="p-4 border rounded-lg bg-green-500/10 border-green-500/20">
                        <div class="flex items-center space-x-2 mb-2">
                            <span class="w-2 h-2 rounded-full bg-green-500"></span>
                            <span class="text-xs font-medium text-green-400">"Connected"</span>
                        </div>
                        <p class="text-sm font-medium text-white">"Discord"</p>
                    </div>
                    <div class="p-4 border rounded-lg bg-slate-500/10 border-slate-500/20">
                        <div class="flex items-center space-x-2 mb-2">
                            <span class="w-2 h-2 rounded-full bg-slate-500"></span>
                            <span class="text-xs font-medium text-slate-400">"Disconnected"</span>
                        </div>
                        <p class="text-sm font-medium text-white">"WhatsApp"</p>
                    </div>
                    <div class="p-4 border rounded-lg bg-yellow-500/10 border-yellow-500/20">
                        <div class="flex items-center space-x-2 mb-2">
                            <span class="w-2 h-2 rounded-full bg-yellow-500"></span>
                            <span class="text-xs font-medium text-yellow-400">"Connecting..."</span>
                        </div>
                        <p class="text-sm font-medium text-white">"Slack"</p>
                    </div>
                    <div class="p-4 border rounded-lg bg-red-500/10 border-red-500/20">
                        <div class="flex items-center space-x-2 mb-2">
                            <span class="w-2 h-2 rounded-full bg-red-500"></span>
                            <span class="text-xs font-medium text-red-400">"Error"</span>
                        </div>
                        <p class="text-sm font-medium text-white">"LINE"</p>
                    </div>
                    <div class="p-4 border rounded-lg bg-slate-500/10 border-slate-500/20">
                        <div class="flex items-center space-x-2 mb-2">
                            <span class="w-2 h-2 rounded-full bg-slate-500"></span>
                            <span class="text-xs font-medium text-slate-400">"Disconnected"</span>
                        </div>
                        <p class="text-sm font-medium text-white">"Matrix"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Channels page component
#[component]
fn ChannelsPage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <div>
                    <h2 class="text-3xl font-bold text-white">"Channels"</h2>
                    <p class="text-slate-400 mt-1">"Manage your messaging channels and integrations"</p>
                </div>
                <button class="px-4 py-2.5 bg-indigo-500 hover:bg-indigo-600 text-white font-medium rounded-lg transition-colors flex items-center space-x-2">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                    </svg>
                    <span>"Add Channel"</span>
                </button>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="flex items-center space-x-3">
                            <div class="w-12 h-12 bg-blue-500 rounded-xl flex items-center justify-center">
                                <span class="text-lg font-bold text-white">"T"</span>
                            </div>
                            <div>
                                <h4 class="font-medium text-white">"Telegram"</h4>
                                <p class="text-xs text-slate-400">"Connect with Telegram users and groups"</p>
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <span class="w-2 h-2 rounded-full bg-green-500"></span>
                            <span class="text-sm font-medium text-green-400">"Connected"</span>
                        </div>
                        <A href="/channels/telegram" class="text-sm text-indigo-400 hover:text-indigo-300">"Configure"</A>
                    </div>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="flex items-center space-x-3">
                            <div class="w-12 h-12 bg-indigo-500 rounded-xl flex items-center justify-center">
                                <span class="text-lg font-bold text-white">"D"</span>
                            </div>
                            <div>
                                <h4 class="font-medium text-white">"Discord"</h4>
                                <p class="text-xs text-slate-400">"Bot integration for Discord servers"</p>
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <span class="w-2 h-2 rounded-full bg-green-500"></span>
                            <span class="text-sm font-medium text-green-400">"Connected"</span>
                        </div>
                        <A href="/channels/discord" class="text-sm text-indigo-400 hover:text-indigo-300">"Configure"</A>
                    </div>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="flex items-center space-x-3">
                            <div class="w-12 h-12 bg-green-500 rounded-xl flex items-center justify-center">
                                <span class="text-lg font-bold text-white">"W"</span>
                            </div>
                            <div>
                                <h4 class="font-medium text-white">"WhatsApp"</h4>
                                <p class="text-xs text-slate-400">"Twilio-powered WhatsApp messaging"</p>
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <span class="w-2 h-2 rounded-full bg-slate-500"></span>
                            <span class="text-sm font-medium text-slate-400">"Disconnected"</span>
                        </div>
                        <A href="/channels/whatsapp" class="text-sm text-indigo-400 hover:text-indigo-300">"Configure"</A>
                    </div>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="flex items-center space-x-3">
                            <div class="w-12 h-12 bg-purple-500 rounded-xl flex items-center justify-center">
                                <span class="text-lg font-bold text-white">"S"</span>
                            </div>
                            <div>
                                <h4 class="font-medium text-white">"Slack"</h4>
                                <p class="text-xs text-slate-400">"Team communication platform"</p>
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <span class="w-2 h-2 rounded-full bg-yellow-500"></span>
                            <span class="text-sm font-medium text-yellow-400">"Connecting..."</span>
                        </div>
                        <A href="/channels/slack" class="text-sm text-indigo-400 hover:text-indigo-300">"Configure"</A>
                    </div>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="flex items-center space-x-3">
                            <div class="w-12 h-12 bg-green-400 rounded-xl flex items-center justify-center">
                                <span class="text-lg font-bold text-white">"L"</span>
                            </div>
                            <div>
                                <h4 class="font-medium text-white">"LINE"</h4>
                                <p class="text-xs text-slate-400">"LINE Messaging API integration"</p>
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <span class="w-2 h-2 rounded-full bg-red-500"></span>
                            <span class="text-sm font-medium text-red-400">"Error"</span>
                        </div>
                        <A href="/channels/line" class="text-sm text-indigo-400 hover:text-indigo-300">"Configure"</A>
                    </div>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 hover:border-slate-600/50 transition-all">
                    <div class="flex items-center justify-between mb-4">
                        <div class="flex items-center space-x-3">
                            <div class="w-12 h-12 bg-slate-600 rounded-xl flex items-center justify-center">
                                <span class="text-lg font-bold text-white">"M"</span>
                            </div>
                            <div>
                                <h4 class="font-medium text-white">"Matrix"</h4>
                                <p class="text-xs text-slate-400">"Decentralized messaging"</p>
                            </div>
                        </div>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <span class="w-2 h-2 rounded-full bg-slate-500"></span>
                            <span class="text-sm font-medium text-slate-400">"Disconnected"</span>
                        </div>
                        <A href="/channels/matrix" class="text-sm text-indigo-400 hover:text-indigo-300">"Configure"</A>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Channel detail page component
#[component]
fn ChannelDetailPage() -> impl IntoView {
    view! {
        <div class="max-w-2xl mx-auto space-y-6">
            <div class="flex items-center space-x-4">
                <A href="/channels" class="p-2 hover:bg-slate-700/50 rounded-lg transition-colors">
                    <svg class="w-5 h-5 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"></path>
                    </svg>
                </A>
                <div>
                    <h2 class="text-2xl font-bold text-white">"Channel Settings"</h2>
                    <p class="text-slate-400">"Configure your Telegram integration"</p>
                </div>
            </div>

            <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 space-y-6">
                <div class="flex items-center justify-between">
                    <div>
                        <label class="text-sm font-medium text-white">"Enable Webhook"</label>
                        <p class="text-xs text-slate-400 mt-0.5">"Receive updates via webhook instead of polling"</p>
                    </div>
                    <button class="w-12 h-6 bg-indigo-500 rounded-full transition-colors relative">
                        <span class="absolute top-1 left-7 w-4 h-4 bg-white rounded-full"></span>
                    </button>
                </div>
                <div class="flex items-center justify-between">
                    <div>
                        <label class="text-sm font-medium text-white">"Auto-reply"</label>
                        <p class="text-xs text-slate-400 mt-0.5">"Automatically respond to messages"</p>
                    </div>
                    <button class="w-12 h-6 bg-slate-600 rounded-full transition-colors relative">
                        <span class="absolute top-1 left-1 w-4 h-4 bg-white rounded-full"></span>
                    </button>
                </div>
                <div class="flex items-center justify-between">
                    <div>
                        <label class="text-sm font-medium text-white">"Message History"</label>
                        <p class="text-xs text-slate-400 mt-0.5">"Store message history for context"</p>
                    </div>
                    <button class="w-12 h-6 bg-indigo-500 rounded-full transition-colors relative">
                        <span class="absolute top-1 left-7 w-4 h-4 bg-white rounded-full"></span>
                    </button>
                </div>
                <div class="flex items-center justify-between">
                    <div>
                        <label class="text-sm font-medium text-white">"Typing Indicators"</label>
                        <p class="text-xs text-slate-400 mt-0.5">"Show typing indicators"</p>
                    </div>
                    <button class="w-12 h-6 bg-indigo-500 rounded-full transition-colors relative">
                        <span class="absolute top-1 left-7 w-4 h-4 bg-white rounded-full"></span>
                    </button>
                </div>

                <div class="pt-4 border-t border-slate-700/50">
                    <h3 class="text-lg font-medium text-white mb-4">"API Configuration"</h3>
                    <div class="space-y-4">
                        <div class="space-y-2">
                            <label class="text-sm font-medium text-white">"Bot Token"</label>
                            <input type="text" placeholder="Enter your Telegram bot token" class="w-full px-4 py-2.5 bg-slate-700/50 border border-slate-600/50 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 transition-colors" />
                        </div>
                        <div class="space-y-2">
                            <label class="text-sm font-medium text-white">"Webhook URL"</label>
                            <input type="text" placeholder="https://your-domain.com/webhook/telegram" class="w-full px-4 py-2.5 bg-slate-700/50 border border-slate-600/50 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 transition-colors" />
                        </div>
                        <div class="space-y-2">
                            <label class="text-sm font-medium text-white">"Secret Token"</label>
                            <input type="text" placeholder="Optional secret token" class="w-full px-4 py-2.5 bg-slate-700/50 border border-slate-600/50 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 transition-colors" />
                        </div>
                    </div>
                </div>

                <div class="flex items-center justify-between pt-4 border-t border-slate-700/50">
                    <button class="px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-400 font-medium rounded-lg transition-colors">
                        "Disconnect"
                    </button>
                    <button class="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white font-medium rounded-lg transition-colors">
                        "Save Changes"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Agents page component
#[component]
fn AgentsPage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <div>
                    <h2 class="text-3xl font-bold text-white">"Agents"</h2>
                    <p class="text-slate-400 mt-1">"Manage your AI agents and automation"</p>
                </div>
                <button class="px-4 py-2.5 bg-indigo-500 hover:bg-indigo-600 text-white font-medium rounded-lg transition-colors flex items-center space-x-2">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
                    </svg>
                    <span>"Create Agent"</span>
                </button>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6">
                    <div class="flex items-start justify-between mb-4">
                        <div>
                            <h4 class="text-lg font-medium text-white">"Summary Agent"</h4>
                            <p class="text-sm text-slate-400 mt-1">"Summarizes messages from all channels"</p>
                        </div>
                        <span class="px-2.5 py-1 text-xs font-medium rounded-full text-green-400 bg-green-500/10">"running"</span>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex flex-wrap gap-2">
                            <span class="px-2 py-1 text-xs bg-slate-700/50 text-slate-300 rounded">"Telegram"</span>
                            <span class="px-2 py-1 text-xs bg-slate-700/50 text-slate-300 rounded">"Discord"</span>
                            <span class="px-2 py-1 text-xs bg-slate-700/50 text-slate-300 rounded">"WhatsApp"</span>
                        </div>
                        <div class="flex items-center space-x-2">
                            <button class="p-2 hover:bg-slate-700/50 rounded-lg transition-colors text-slate-400 hover:text-white">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path>
                                </svg>
                            </button>
                            <button class="p-2 hover:bg-slate-700/50 rounded-lg transition-colors text-slate-400 hover:text-white">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6">
                    <div class="flex items-start justify-between mb-4">
                        <div>
                            <h4 class="text-lg font-medium text-white">"Weather Agent"</h4>
                            <p class="text-sm text-slate-400 mt-1">"Provides weather updates on request"</p>
                        </div>
                        <span class="px-2.5 py-1 text-xs font-medium rounded-full text-green-400 bg-green-500/10">"running"</span>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex flex-wrap gap-2">
                            <span class="px-2 py-1 text-xs bg-slate-700/50 text-slate-300 rounded">"Telegram"</span>
                            <span class="px-2 py-1 text-xs bg-slate-700/50 text-slate-300 rounded">"LINE"</span>
                        </div>
                        <div class="flex items-center space-x-2">
                            <button class="p-2 hover:bg-slate-700/50 rounded-lg transition-colors text-slate-400 hover:text-white">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path>
                                </svg>
                            </button>
                            <button class="p-2 hover:bg-slate-700/50 rounded-lg transition-colors text-slate-400 hover:text-white">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>

                <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6">
                    <div class="flex items-start justify-between mb-4">
                        <div>
                            <h4 class="text-lg font-medium text-white">"Translation Agent"</h4>
                            <p class="text-sm text-slate-400 mt-1">"Auto-translates messages between languages"</p>
                        </div>
                        <span class="px-2.5 py-1 text-xs font-medium rounded-full text-slate-400 bg-slate-500/10">"stopped"</span>
                    </div>
                    <div class="flex items-center justify-between">
                        <div class="flex flex-wrap gap-2">
                            <span class="px-2 py-1 text-xs bg-slate-700/50 text-slate-300 rounded">"Discord"</span>
                        </div>
                        <div class="flex items-center space-x-2">
                            <button class="p-2 hover:bg-slate-700/50 rounded-lg transition-colors text-slate-400 hover:text-white">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path>
                                </svg>
                            </button>
                            <button class="p-2 hover:bg-slate-700/50 rounded-lg transition-colors text-slate-400 hover:text-white">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Logs page component
#[component]
fn LogsPage() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <div>
                    <h2 class="text-3xl font-bold text-white">"Logs"</h2>
                    <p class="text-slate-400 mt-1">"View and filter system logs"</p>
                </div>
                <div class="flex items-center space-x-3">
                    <select class="px-4 py-2 bg-slate-700/50 border border-slate-600/50 rounded-lg text-white text-sm focus:outline-none focus:border-indigo-500">
                        <option>"All Channels"</option>
                        <option>"Telegram"</option>
                        <option>"Discord"</option>
                        <option>"WhatsApp"</option>
                    </select>
                    <select class="px-4 py-2 bg-slate-700/50 border border-slate-600/50 rounded-lg text-white text-sm focus:outline-none focus:border-indigo-500">
                        <option>"All Levels"</option>
                        <option>"ERROR"</option>
                        <option>"WARN"</option>
                        <option>"INFO"</option>
                        <option>"DEBUG"</option>
                    </select>
                </div>
            </div>

            <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl overflow-hidden">
                <div class="overflow-x-auto">
                    <table class="w-full">
                        <thead class="bg-slate-700/30 border-b border-slate-700/50">
                            <tr>
                                <th class="px-4 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Timestamp"</th>
                                <th class="px-4 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Level"</th>
                                <th class="px-4 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Source"</th>
                                <th class="px-4 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Message"</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-slate-700/30">
                            <tr class="hover:bg-slate-700/20 transition-colors">
                                <td class="px-4 py-3 text-sm text-slate-400 font-mono">"2024-01-15 14:32:01"</td>
                                <td class="px-4 py-3"><span class="px-2 py-0.5 text-xs font-medium rounded text-green-400 bg-green-500/10">"INFO"</span></td>
                                <td class="px-4 py-3 text-sm text-slate-300">"telegram"</td>
                                <td class="px-4 py-3 text-sm text-slate-300">"Bot started successfully"</td>
                            </tr>
                            <tr class="hover:bg-slate-700/20 transition-colors">
                                <td class="px-4 py-3 text-sm text-slate-400 font-mono">"2024-01-15 14:32:05"</td>
                                <td class="px-4 py-3"><span class="px-2 py-0.5 text-xs font-medium rounded text-green-400 bg-green-500/10">"INFO"</span></td>
                                <td class="px-4 py-3 text-sm text-slate-300">"discord"</td>
                                <td class="px-4 py-3 text-sm text-slate-300">"Connected to Discord gateway"</td>
                            </tr>
                            <tr class="hover:bg-slate-700/20 transition-colors">
                                <td class="px-4 py-3 text-sm text-slate-400 font-mono">"2024-01-15 14:32:10"</td>
                                <td class="px-4 py-3"><span class="px-2 py-0.5 text-xs font-medium rounded text-yellow-400 bg-yellow-500/10">"WARN"</span></td>
                                <td class="px-4 py-3 text-sm text-slate-300">"whatsapp"</td>
                                <td class="px-4 py-3 text-sm text-slate-300">"Failed to connect, retrying..."</td>
                            </tr>
                            <tr class="hover:bg-slate-700/20 transition-colors">
                                <td class="px-4 py-3 text-sm text-slate-400 font-mono">"2024-01-15 14:32:15"</td>
                                <td class="px-4 py-3"><span class="px-2 py-0.5 text-xs font-medium rounded text-red-400 bg-red-500/10">"ERROR"</span></td>
                                <td class="px-4 py-3 text-sm text-slate-300">"whatsapp"</td>
                                <td class="px-4 py-3 text-sm text-slate-300">"Connection timeout after 3 attempts"</td>
                            </tr>
                            <tr class="hover:bg-slate-700/20 transition-colors">
                                <td class="px-4 py-3 text-sm text-slate-400 font-mono">"2024-01-15 14:32:20"</td>
                                <td class="px-4 py-3"><span class="px-2 py-0.5 text-xs font-medium rounded text-green-400 bg-green-500/10">"INFO"</span></td>
                                <td class="px-4 py-3 text-sm text-slate-300">"telegram"</td>
                                <td class="px-4 py-3 text-sm text-slate-300">"Received /start command from user 12345"</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    }
}

/// Settings page component
#[component]
fn SettingsPage() -> impl IntoView {
    view! {
        <div class="max-w-2xl mx-auto space-y-6">
            <div>
                <h2 class="text-3xl font-bold text-white">"Settings"</h2>
                <p class="text-slate-400 mt-1">"Configure your gateway settings"</p>
            </div>

            <div class="bg-slate-800/30 border border-slate-700/50 rounded-xl p-6 space-y-6">
                <h3 class="text-lg font-medium text-white">"General"</h3>
                <div class="space-y-4">
                    <div class="space-y-2">
                        <label class="text-sm font-medium text-white">"Gateway Name"</label>
                        <input type="text" placeholder="My Carapace Gateway" value="My Gateway" class="w-full px-4 py-2.5 bg-slate-700/50 border border-slate-600/50 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 transition-colors" />
                    </div>
                    <div class="space-y-2">
                        <label class="text-sm font-medium text-white">"Admin Email"</label>
                        <input type="text" placeholder="admin@example.com" class="w-full px-4 py-2.5 bg-slate-700/50 border border-slate-600/50 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 transition-colors" />
                    </div>
                </div>

                <div class="pt-4 border-t border-slate-700/50">
                    <h3 class="text-lg font-medium text-white mb-4">"Security"</h3>
                    <div class="space-y-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <label class="text-sm font-medium text-white">"Enable Authentication"</label>
                                <p class="text-xs text-slate-400 mt-0.5">"Require login to access the dashboard"</p>
                            </div>
                            <button class="w-12 h-6 bg-indigo-500 rounded-full transition-colors relative">
                                <span class="absolute top-1 left-7 w-4 h-4 bg-white rounded-full"></span>
                            </button>
                        </div>
                        <div class="flex items-center justify-between">
                            <div>
                                <label class="text-sm font-medium text-white">"API Authentication"</label>
                                <p class="text-xs text-slate-400 mt-0.5">"Require API keys for external access"</p>
                            </div>
                            <button class="w-12 h-6 bg-indigo-500 rounded-full transition-colors relative">
                                <span class="absolute top-1 left-7 w-4 h-4 bg-white rounded-full"></span>
                            </button>
                        </div>
                    </div>
                </div>

                <div class="pt-4 border-t border-slate-700/50">
                    <h3 class="text-lg font-medium text-white mb-4">"Storage"</h3>
                    <div class="space-y-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <label class="text-sm font-medium text-white">"Persist Messages"</label>
                                <p class="text-xs text-slate-400 mt-0.5">"Store messages to disk"</p>
                            </div>
                            <button class="w-12 h-6 bg-indigo-500 rounded-full transition-colors relative">
                                <span class="absolute top-1 left-7 w-4 h-4 bg-white rounded-full"></span>
                            </button>
                        </div>
                        <div class="flex items-center justify-between">
                            <div>
                                <label class="text-sm font-medium text-white">"Persist Sessions"</label>
                                <p class="text-xs text-slate-400 mt-0.5">"Store session data to disk"</p>
                            </div>
                            <button class="w-12 h-6 bg-indigo-500 rounded-full transition-colors relative">
                                <span class="absolute top-1 left-7 w-4 h-4 bg-white rounded-full"></span>
                            </button>
                        </div>
                    </div>
                </div>

                <div class="flex justify-end pt-4 border-t border-slate-700/50">
                    <button class="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white font-medium rounded-lg transition-colors">
                        "Save Settings"
                    </button>
                </div>
            </div>
        </div>
    }
}
