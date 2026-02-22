use leptos::either::Either;
use leptos::prelude::*;

use common::models::{FilterProfile, SystemFilter, ToolFilter};
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

pub fn render_filters_index(profiles: &[FilterProfile], active_profile_id: &str) -> String {
    let profiles = profiles.to_vec();
    let empty = profiles.is_empty();
    let total = profiles.len();
    let active_profile_id = active_profile_id.to_string();

    let content = view! {
        <h2>"Profiles"</h2>
        <p>{format!("Total: {}", total)}</p>
        {if empty {
            Either::Left(view! {
                <p>"No profiles yet."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"ID"</th>
                        <th>"Name"</th>
                        <th>"Status"</th>
                        <th>"Created"</th>
                        <th></th>
                    </tr>
                    {profiles.into_iter().map(|p| {
                        let pid = p.id.to_string();
                        let is_active = pid == active_profile_id;
                        let href = format!("/_dashboard/filters/{}", pid);
                        let activate_action = format!("/_dashboard/filters/{}/activate", pid);
                        let delete_action = format!("/_dashboard/filters/{}/delete", pid);
                        view! {
                            <tr>
                                <td><a href={href}>{pid}</a></td>
                                <td>{p.name}</td>
                                <td>{if is_active { "active" } else { "--" }}</td>
                                <td>{p.created_at.clone().unwrap_or_default()}</td>
                                <td>
                                    {if !is_active {
                                        Either::Left(view! {
                                            <form method="POST" action={activate_action}>
                                                <button type="submit">"Activate"</button>
                                            </form>
                                            " "
                                        })
                                    } else {
                                        Either::Right(())
                                    }}
                                    <form method="POST" action={delete_action}>
                                        <button type="submit">"Delete"</button>
                                    </form>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        }}
    };

    Page {
        title: "Gateway Proxy - Filters".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::current("Filters"),
        ],
        nav_links: vec![
            NavLink::new("New Profile", "/_dashboard/filters/new"),
            NavLink::back(),
        ],
        info_rows: vec![],
        content,
        subpages: vec![],
    }
    .render()
}

pub fn render_new_profile() -> String {
    let form = view! {
        <h2>"New Profile"</h2>
        <form method="POST" action="/_dashboard/filters/new">
            <table>
                <tr>
                    <td><label>"Name"</label></td>
                    <td><input type="text" name="name" required size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Create"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: "Gateway Proxy - New Profile".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::current("New Profile"),
        ],
        nav_links: vec![NavLink::back()],
        content: form,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_profile_show(
    profile: &FilterProfile,
    active_profile_id: &str,
    system_count: i64,
    tool_count: i64,
    keep_tool_pairs: i64,
) -> String {
    let profile = profile.clone();
    let is_active = profile.id.to_string() == active_profile_id;
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let activate_action = format!("/_dashboard/filters/{}/activate", profile_id);
    let message_filter_label = if keep_tool_pairs > 0 {
        format!("keep last {}", keep_tool_pairs)
    } else {
        "off".to_string()
    };

    let actions = if !is_active {
        Either::Left(view! {
            <h2>"Actions"</h2>
            <form method="POST" action={activate_action}>
                <button type="submit">"Activate"</button>
            </form>
        })
    } else {
        Either::Right(())
    };

    Page {
        title: format!("Gateway Proxy - Profile {}", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::current(format!("Profile {}", profile_name)),
        ],
        nav_links: vec![
            NavLink::new(
                "Edit Profile",
                format!("/_dashboard/filters/{}/edit", profile_id),
            ),
            NavLink::back(),
        ],
        info_rows: vec![
            InfoRow::new("Name", &profile.name),
            InfoRow::new("Status", if is_active { "active" } else { "inactive" }),
            InfoRow::new("Created", &profile.created_at.unwrap_or_default()),
        ],
        content: actions,
        subpages: vec![
            Subpage::new(
                "System Filters",
                format!("/_dashboard/filters/{}/system", profile_id),
                system_count,
            ),
            Subpage::new(
                "Tool Filters",
                format!("/_dashboard/filters/{}/tools", profile_id),
                tool_count,
            ),
            Subpage::new(
                "Message Filters",
                format!("/_dashboard/filters/{}/messages", profile_id),
                message_filter_label,
            ),
        ],
    }
    .render()
}

pub fn render_profile_edit(profile: &FilterProfile) -> String {
    let profile = profile.clone();
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let edit_action = format!("/_dashboard/filters/{}/edit", profile_id);

    let form = view! {
        <h2>"Edit Profile"</h2>
        <form method="POST" action={edit_action}>
            <table>
                <tr>
                    <td><label>"Name"</label></td>
                    <td><input type="text" name="name" required value={profile.name} size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: format!("Gateway Proxy - Edit Profile {}", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::current("Edit"),
        ],
        nav_links: vec![NavLink::back()],
        content: form,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_profile_system(profile: &FilterProfile, system_filters: &[SystemFilter]) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let system_filters = system_filters.to_vec();
    let system_total = system_filters.len();
    let system_empty = system_filters.is_empty();

    let content = view! {
        <h2>"System Filters"</h2>
        <p>{format!("Total: {}", system_total)}</p>
        {if system_empty {
            Either::Left(view! {
                <p>"No system filters configured."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"ID"</th>
                        <th>"Pattern"</th>
                        <th>"Created"</th>
                        <th></th>
                    </tr>
                    {system_filters.into_iter().map(|f| {
                        let edit_href = format!("/_dashboard/filters/{}/system/{}/edit", profile_id, f.id);
                        let delete_action = format!("/_dashboard/filters/{}/system/{}/delete", profile_id, f.id);
                        let id_str = f.id.to_string();
                        view! {
                            <tr>
                                <td>{id_str}</td>
                                <td>{f.pattern}</td>
                                <td>{f.created_at.clone().unwrap_or_default()}</td>
                                <td>
                                    <a href={edit_href}>"Edit"</a>
                                    " "
                                    <form method="POST" action={delete_action}>
                                        <button type="submit">"Delete"</button>
                                    </form>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        }}
    };

    Page {
        title: format!("Gateway Proxy - {} System Filters", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::current("System Filters"),
        ],
        nav_links: vec![
            NavLink::new(
                "New System Filter",
                format!("/_dashboard/filters/{}/system/new", profile_id),
            ),
            NavLink::back(),
        ],
        content,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_profile_system_new(
    profile: &FilterProfile,
    system_filters: &[SystemFilter],
) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let form_action = format!("/_dashboard/filters/{}/system", profile_id);

    let existing_patterns: Vec<String> = system_filters.iter().map(|f| f.pattern.clone()).collect();
    let system_suggestions: Vec<&&str> = db::DEFAULT_FILTER_SUGGESTIONS
        .iter()
        .filter(|s| !existing_patterns.contains(&s.to_string()))
        .collect();
    let has_suggestions = !system_suggestions.is_empty();

    let content = view! {
        <h2>"New System Filter"</h2>
        <form method="POST" action={form_action.clone()}>
            <table>
                <tr>
                    <td><label>"Pattern"</label></td>
                    <td><input type="text" name="pattern" required size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Add Filter"/></td>
                </tr>
            </table>
        </form>
        {if has_suggestions {
            Either::Left(view! {
                <h2>"Suggested System Filters"</h2>
                <table>
                    {system_suggestions.into_iter().map(|s| {
                        let pattern = s.to_string();
                        view! {
                            <tr>
                                <td><code>{pattern.clone()}</code></td>
                                <td>
                                    <form method="POST" action={form_action.clone()}>
                                        <input type="hidden" name="pattern" value={pattern}/>
                                        <button type="submit">"Add"</button>
                                    </form>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        } else {
            Either::Right(())
        }}
    };

    Page {
        title: format!("Gateway Proxy - {} New System Filter", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::link(
                "System Filters",
                format!("/_dashboard/filters/{}/system", profile_id),
            ),
            Breadcrumb::current("New"),
        ],
        nav_links: vec![NavLink::back()],
        content,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_profile_tools(profile: &FilterProfile, tool_filters: &[ToolFilter]) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let tool_filters = tool_filters.to_vec();
    let tool_total = tool_filters.len();
    let tool_empty = tool_filters.is_empty();

    let content = view! {
        <h2>"Tool Filters"</h2>
        <p>{format!("Total: {}", tool_total)}</p>
        {if tool_empty {
            Either::Left(view! {
                <p>"No tool filters configured."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"ID"</th>
                        <th>"Tool Name"</th>
                        <th>"Created"</th>
                        <th></th>
                    </tr>
                    {tool_filters.into_iter().map(|f| {
                        let edit_href = format!("/_dashboard/filters/{}/tools/{}/edit", profile_id, f.id);
                        let delete_action = format!("/_dashboard/filters/{}/tools/{}/delete", profile_id, f.id);
                        let id_str = f.id.to_string();
                        view! {
                            <tr>
                                <td>{id_str}</td>
                                <td>{f.name}</td>
                                <td>{f.created_at.clone().unwrap_or_default()}</td>
                                <td>
                                    <a href={edit_href}>"Edit"</a>
                                    " "
                                    <form method="POST" action={delete_action}>
                                        <button type="submit">"Delete"</button>
                                    </form>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        }}
    };

    Page {
        title: format!("Gateway Proxy - {} Tool Filters", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::current("Tool Filters"),
        ],
        nav_links: vec![
            NavLink::new(
                "New Tool Filter",
                format!("/_dashboard/filters/{}/tools/new", profile_id),
            ),
            NavLink::back(),
        ],
        content,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_profile_tools_new(profile: &FilterProfile, tool_filters: &[ToolFilter]) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let form_action = format!("/_dashboard/filters/{}/tools", profile_id);

    let existing_names: Vec<String> = tool_filters.iter().map(|f| f.name.clone()).collect();
    let tool_suggestions: Vec<&&str> = db::DEFAULT_TOOL_FILTER_SUGGESTIONS
        .iter()
        .filter(|s| !existing_names.contains(&s.to_string()))
        .collect();
    let has_suggestions = !tool_suggestions.is_empty();

    let content = view! {
        <h2>"New Tool Filter"</h2>
        <form method="POST" action={form_action.clone()}>
            <table>
                <tr>
                    <td><label>"Tool Name"</label></td>
                    <td><input type="text" name="name" required size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Add Filter"/></td>
                </tr>
            </table>
        </form>
        {if has_suggestions {
            Either::Left(view! {
                <h2>"Suggested Tool Filters"</h2>
                <table>
                    {tool_suggestions.into_iter().map(|s| {
                        let name = s.to_string();
                        view! {
                            <tr>
                                <td><code>{name.clone()}</code></td>
                                <td>
                                    <form method="POST" action={form_action.clone()}>
                                        <input type="hidden" name="name" value={name}/>
                                        <button type="submit">"Add"</button>
                                    </form>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </table>
            })
        } else {
            Either::Right(())
        }}
    };

    Page {
        title: format!("Gateway Proxy - {} New Tool Filter", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::link(
                "Tool Filters",
                format!("/_dashboard/filters/{}/tools", profile_id),
            ),
            Breadcrumb::current("New"),
        ],
        nav_links: vec![NavLink::back()],
        content,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_system_filter_edit(profile: &FilterProfile, filter: &SystemFilter) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let filter_id = filter.id.to_string();
    let edit_action = format!(
        "/_dashboard/filters/{}/system/{}/edit",
        profile_id, filter_id
    );

    let form = view! {
        <h2>"Edit System Filter"</h2>
        <form method="POST" action={edit_action}>
            <table>
                <tr>
                    <td><label>"Pattern"</label></td>
                    <td><input type="text" name="pattern" required value={filter.pattern.clone()} size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: "Gateway Proxy - Edit System Filter".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::link(
                "System Filters",
                format!("/_dashboard/filters/{}/system", profile_id),
            ),
            Breadcrumb::current("Edit"),
        ],
        nav_links: vec![NavLink::back()],
        content: form,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_tool_filter_edit(profile: &FilterProfile, filter: &ToolFilter) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let filter_id = filter.id.to_string();
    let edit_action = format!(
        "/_dashboard/filters/{}/tools/{}/edit",
        profile_id, filter_id
    );

    let form = view! {
        <h2>"Edit Tool Filter"</h2>
        <form method="POST" action={edit_action}>
            <table>
                <tr>
                    <td><label>"Tool Name"</label></td>
                    <td><input type="text" name="name" required value={filter.name.clone()} size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: "Gateway Proxy - Edit Tool Filter".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::link(
                "Tool Filters",
                format!("/_dashboard/filters/{}/tools", profile_id),
            ),
            Breadcrumb::current("Edit"),
        ],
        nav_links: vec![NavLink::back()],
        content: form,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_profile_messages(profile: &FilterProfile, keep_tool_pairs: i64) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let form_action = format!("/_dashboard/filters/{}/messages", profile_id);

    let content = view! {
        <h2>"Message Filters"</h2>
        <p>"Controls how many tool_use/tool_result pairs to keep in forwarded requests. Set to 0 to disable (keep all)."</p>
        <form method="POST" action={form_action}>
            <table>
                <tr>
                    <td><label>"Keep last N tool pairs"</label></td>
                    <td><input type="number" name="keep_tool_pairs" min="0" value={keep_tool_pairs.to_string()} size="10"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: format!("Gateway Proxy - {} Message Filters", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::current("Message Filters"),
        ],
        nav_links: vec![NavLink::back()],
        content,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}
