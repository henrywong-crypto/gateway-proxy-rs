use common::models::{
    FilterProfile, SystemFilter, ToolFilter, ToolNameOverride, DEFAULT_SYSTEM_FILTER_SUGGESTIONS,
    DEFAULT_TOOL_FILTER_SUGGESTIONS,
};
use leptos::{either::Either, prelude::*};
use templates::{Breadcrumb, InfoRow, NavLink, Page, Subpage};

pub fn render_filters_view(profiles: &[FilterProfile]) -> String {
    let profiles = profiles.to_vec();
    let empty = profiles.is_empty();
    let total = profiles.len();

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
                        <th>"Created"</th>
                        <th></th>
                    </tr>
                    {profiles.into_iter().map(|profile| {
                        let profile_id = profile.id.to_string();
                        let href = format!("/_dashboard/filters/{}", profile_id);
                        let delete_action = format!("/_dashboard/filters/{}/delete", profile_id);
                        let is_default = profile.is_default;
                        let name_display = if is_default {
                            format!("{} (default)", profile.name)
                        } else {
                            profile.name.clone()
                        };
                        view! {
                            <tr>
                                <td><a href={href}>{profile_id}</a></td>
                                <td>{name_display}</td>
                                <td>{profile.created_at.clone()}</td>
                                <td>
                                    {if !is_default {
                                        Either::Left(view! {
                                            <form method="POST" action={delete_action}>
                                                <button type="submit">"Delete"</button>
                                            </form>
                                        })
                                    } else {
                                        Either::Right(())
                                    }}
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

pub fn render_new_profile_form() -> String {
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

pub fn render_profile_view(
    profile: &FilterProfile,
    system_count: i64,
    tool_count: i64,
    keep_tool_pairs: i64,
    override_count: i64,
) -> String {
    let profile = profile.clone();
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let message_filter_label = if keep_tool_pairs > 0 {
        format!("keep last {}", keep_tool_pairs)
    } else {
        "off".to_string()
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
            InfoRow::new("Default", if profile.is_default { "yes" } else { "no" }),
            InfoRow::new("Created", &profile.created_at),
        ],
        content: (),
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
            Subpage::new(
                "Tool Name Overrides",
                format!("/_dashboard/filters/{}/tool-name-overrides", profile_id),
                override_count,
            ),
        ],
    }
    .render()
}

pub fn render_edit_profile_form(profile: &FilterProfile) -> String {
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

pub fn render_system_filters_view(
    profile: &FilterProfile,
    system_filters: &[SystemFilter],
) -> String {
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
                    {system_filters.into_iter().map(|filter| {
                        let edit_href = format!("/_dashboard/filters/{}/system/{}/edit", profile_id, filter.id);
                        let delete_action = format!("/_dashboard/filters/{}/system/{}/delete", profile_id, filter.id);
                        let id_str = filter.id.to_string();
                        view! {
                            <tr>
                                <td>{id_str}</td>
                                <td>{filter.pattern}</td>
                                <td>{filter.created_at.clone()}</td>
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

pub fn render_new_system_filter_form(
    profile: &FilterProfile,
    system_filters: &[SystemFilter],
) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let form_action = format!("/_dashboard/filters/{}/system", profile_id);

    let existing_patterns: Vec<String> = system_filters.iter().map(|filter| filter.pattern.clone()).collect();
    let system_suggestions: Vec<&&str> = DEFAULT_SYSTEM_FILTER_SUGGESTIONS
        .iter()
        .filter(|suggestion| !existing_patterns.contains(&suggestion.to_string()))
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
                    {system_suggestions.into_iter().map(|suggestion| {
                        let pattern = suggestion.to_string();
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

pub fn render_tool_filters_view(profile: &FilterProfile, tool_filters: &[ToolFilter]) -> String {
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
                    {tool_filters.into_iter().map(|filter| {
                        let edit_href = format!("/_dashboard/filters/{}/tools/{}/edit", profile_id, filter.id);
                        let delete_action = format!("/_dashboard/filters/{}/tools/{}/delete", profile_id, filter.id);
                        let id_str = filter.id.to_string();
                        view! {
                            <tr>
                                <td>{id_str}</td>
                                <td>{filter.name}</td>
                                <td>{filter.created_at.clone()}</td>
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

pub fn render_new_tool_filter_form(profile: &FilterProfile, tool_filters: &[ToolFilter]) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let form_action = format!("/_dashboard/filters/{}/tools", profile_id);

    let existing_names: Vec<String> = tool_filters.iter().map(|filter| filter.name.clone()).collect();
    let tool_suggestions: Vec<&&str> = DEFAULT_TOOL_FILTER_SUGGESTIONS
        .iter()
        .filter(|suggestion| !existing_names.contains(&suggestion.to_string()))
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
                    {tool_suggestions.into_iter().map(|suggestion| {
                        let name = suggestion.to_string();
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

pub fn render_edit_system_filter_form(profile: &FilterProfile, filter: &SystemFilter) -> String {
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

pub fn render_edit_tool_filter_form(profile: &FilterProfile, filter: &ToolFilter) -> String {
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

pub fn render_tool_name_overrides_view(
    profile: &FilterProfile,
    overrides: &[ToolNameOverride],
) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let overrides = overrides.to_vec();
    let total = overrides.len();
    let empty = overrides.is_empty();

    let content = view! {
        <h2>"Tool Name Overrides"</h2>
        <p>{format!("Total: {}", total)}</p>
        {if empty {
            Either::Left(view! {
                <p>"No tool name overrides configured."</p>
            })
        } else {
            Either::Right(view! {
                <table>
                    <tr>
                        <th>"ID"</th>
                        <th>"Original Name"</th>
                        <th>"Override Name"</th>
                        <th>"Created"</th>
                        <th></th>
                    </tr>
                    {overrides.into_iter().map(|o| {
                        let edit_href = format!(
                            "/_dashboard/filters/{}/tool-name-overrides/{}/edit",
                            profile_id, o.id
                        );
                        let delete_action = format!(
                            "/_dashboard/filters/{}/tool-name-overrides/{}/delete",
                            profile_id, o.id
                        );
                        let id_str = o.id.to_string();
                        view! {
                            <tr>
                                <td>{id_str}</td>
                                <td>{o.original_name}</td>
                                <td>{o.override_name}</td>
                                <td>{o.created_at.clone()}</td>
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
        title: format!("Gateway Proxy - {} Tool Name Overrides", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::current("Tool Name Overrides"),
        ],
        nav_links: vec![
            NavLink::new(
                "New Override",
                format!(
                    "/_dashboard/filters/{}/tool-name-overrides/new",
                    profile_id
                ),
            ),
            NavLink::back(),
        ],
        content,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_new_tool_name_override_form(profile: &FilterProfile) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let form_action = format!(
        "/_dashboard/filters/{}/tool-name-overrides",
        profile_id
    );

    let form = view! {
        <h2>"New Tool Name Override"</h2>
        <form method="POST" action={form_action}>
            <table>
                <tr>
                    <td><label>"Original Name"</label></td>
                    <td><input type="text" name="original_name" required size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Override Name"</label></td>
                    <td><input type="text" name="override_name" required size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Add Override"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: format!("Gateway Proxy - {} New Tool Name Override", profile_name),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::link(
                "Tool Name Overrides",
                format!("/_dashboard/filters/{}/tool-name-overrides", profile_id),
            ),
            Breadcrumb::current("New"),
        ],
        nav_links: vec![NavLink::back()],
        content: form,
        info_rows: vec![],
        subpages: vec![],
    }
    .render()
}

pub fn render_edit_tool_name_override_form(
    profile: &FilterProfile,
    tool_name_override: &ToolNameOverride,
) -> String {
    let profile_name = profile.name.clone();
    let profile_id = profile.id.to_string();
    let override_id = tool_name_override.id.to_string();
    let edit_action = format!(
        "/_dashboard/filters/{}/tool-name-overrides/{}/edit",
        profile_id, override_id
    );

    let form = view! {
        <h2>"Edit Tool Name Override"</h2>
        <form method="POST" action={edit_action}>
            <table>
                <tr>
                    <td><label>"Original Name"</label></td>
                    <td><input type="text" name="original_name" required value={tool_name_override.original_name.clone()} size="60"/></td>
                </tr>
                <tr>
                    <td><label>"Override Name"</label></td>
                    <td><input type="text" name="override_name" required value={tool_name_override.override_name.clone()} size="60"/></td>
                </tr>
                <tr>
                    <td></td>
                    <td><input type="submit" value="Save"/></td>
                </tr>
            </table>
        </form>
    };

    Page {
        title: "Gateway Proxy - Edit Tool Name Override".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Home", "/_dashboard"),
            Breadcrumb::link("Filters", "/_dashboard/filters"),
            Breadcrumb::link(
                format!("Profile {}", profile_name),
                format!("/_dashboard/filters/{}", profile_id),
            ),
            Breadcrumb::link(
                "Tool Name Overrides",
                format!("/_dashboard/filters/{}/tool-name-overrides", profile_id),
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

pub fn render_message_filters_view(profile: &FilterProfile, keep_tool_pairs: i64) -> String {
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
