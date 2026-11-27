//! Rune view for the Anyinstaller packager GUI.

use rune::tok::motion;
use rune::{
    col, el, row, spacer, text as rune_text, text_input, Color, Cx, Direction, El, ScrollSpec,
    TextStyle,
};

use crate::app::{MarketDownload, State, DEFAULT_TIMESTAMP_URL};
use crate::build_job::LogKind;
use crate::i18n::{self, Language};
use crate::market::{self, ComponentKind};
use crate::model::{
    BackgroundMaterial, BackgroundMode, BuildPhase, NoticeTone, PathTarget, RegionStatus, SignMode,
    Step, UiField,
};

const DISPLAY: TextStyle = TextStyle::new(22.0, 650.0);
const CARD_TITLE: TextStyle = TextStyle::new(14.0, 600.0);
const BODY: TextStyle = TextStyle::new(12.5, 400.0);
const BODY_MEDIUM: TextStyle = TextStyle::new(12.5, 550.0);
const LABEL: TextStyle = TextStyle::new(11.0, 600.0);
const CAPTION: TextStyle = TextStyle::new(10.5, 450.0);
const MONO: TextStyle = TextStyle::new(11.5, 500.0).mono();
const MONO_SMALL: TextStyle = TextStyle::new(10.5, 450.0).mono();
const BUTTON: TextStyle = TextStyle::new(12.0, 550.0);

#[derive(Clone, Copy)]
struct Paint {
    bg: Color,
    sidebar: Color,
    surface: Color,
    surface_high: Color,
    field: Color,
    border: Color,
    border_strong: Color,
    text: Color,
    muted: Color,
    faint: Color,
    accent: Color,
    accent_hover: Color,
    accent_soft: Color,
    info: Color,
    success: Color,
    warning: Color,
    danger: Color,
    log_bg: Color,
}

impl Paint {
    fn dark() -> Self {
        Self {
            bg: rune::oklch(0.17, 0.004, 250.0),
            sidebar: rune::oklch(0.145, 0.005, 250.0),
            surface: rune::oklch(0.205, 0.005, 250.0),
            surface_high: rune::oklch(0.23, 0.006, 250.0),
            field: rune::oklch(0.165, 0.004, 250.0),
            border: Color::WHITE.alpha(0.065),
            border_strong: Color::WHITE.alpha(0.12),
            text: rune::oklch(0.93, 0.004, 250.0),
            muted: rune::oklch(0.68, 0.008, 250.0),
            faint: rune::oklch(0.48, 0.006, 250.0),
            accent: rune::oklch(0.64, 0.075, 245.0),
            accent_hover: rune::oklch(0.69, 0.075, 245.0),
            accent_soft: rune::oklcha(0.64, 0.075, 245.0, 0.12),
            info: rune::oklch(0.72, 0.055, 235.0),
            success: rune::oklch(0.70, 0.075, 150.0),
            warning: rune::oklch(0.75, 0.09, 85.0),
            danger: rune::oklch(0.66, 0.12, 25.0),
            log_bg: rune::oklch(0.12, 0.004, 250.0),
        }
    }
}

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy)]
enum BoolField {
    StartMenu,
    Desktop,
    Force,
    Acceleration,
}

pub fn view(state: &State, _: &mut Cx<State>) -> El {
    i18n::set_current(state.language);
    let p = Paint::dark();
    let direction = if state.language.is_rtl() {
        Direction::Rtl
    } else {
        Direction::Ltr
    };
    row()
        .dir(direction)
        .w_full()
        .h_full()
        .fill(p.bg)
        .child(sidebar(state, p))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .h_full()
                .child(header(state, p))
                .child(
                    row()
                        .w_full()
                        .grow(1.0)
                        .min_h(0.0)
                        .child(editor(state, p))
                        .child(inspector(state, p)),
                )
                .child(footer(state, p)),
        )
}

fn text(content: impl Into<String>) -> El {
    rune_text(i18n::tr(&content.into()))
}

fn sidebar(state: &State, p: Paint) -> El {
    let mut nav = col().w_full().gap(6.0);
    for step in Step::ALL {
        nav = nav.child(step_nav_item(state, p, step));
    }

    col()
        .w(210.0)
        .h_full()
        .px(16.0)
        .py(18.0)
        .gap(18.0)
        .fill(p.sidebar)
        .border(1.0, p.border)
        .child(
            row()
                .gap(11.0)
                .center_y()
                .child(
                    el().size(32.0, 32.0)
                        .radius(8.0)
                        .fill(p.surface_high)
                        .border(1.0, p.border_strong)
                        .center()
                        .child(text("A").font(TextStyle::new(15.0, 700.0)).color(p.accent)),
                )
                .child(
                    col()
                        .gap(1.0)
                        .child(text("ANYINSTALLER").font(MONO).color(p.text))
                        .child(text("AIP PACKAGER").font(MONO_SMALL).color(p.muted)),
                ),
        )
        .child(el().w_full().h(1.0).fill(p.border))
        .child(
            row()
                .justify_between()
                .center_y()
                .child(text("工作流程").font(MONO_SMALL).color(p.faint))
                .child(pill(p, "AIP v1", p.info)),
        )
        .child(nav)
        .child(spacer().grow(1.0))
        .child(settings_nav_item(state, p))
        .child(
            col()
                .w_full()
                .gap(9.0)
                .padding(13.0)
                .radius(10.0)
                .fill(p.surface)
                .border(1.0, p.border)
                .child(text("Windows 10 / 11").font(MONO_SMALL).color(p.muted))
                .child(
                    row()
                        .gap(7.0)
                        .center_y()
                        .child(dot(p.success, 6.0))
                        .child(text("单文件 · Windows").font(CAPTION).color(p.text)),
                ),
        )
}

fn settings_nav_item(state: &State, p: Paint) -> El {
    let active = state.step == Step::Settings;
    let mut item = row()
        .key("settings-nav")
        .role("button")
        .label(i18n::tr("设置"))
        .focusable(true)
        .w_full()
        .h(44.0)
        .px(10.0)
        .gap(10.0)
        .center_y()
        .radius(9.0)
        .hover(if active {
            p.accent_soft
        } else {
            Color::WHITE.alpha(0.035)
        })
        .on_press_tx(|s: &mut State, _tx| s.open_settings());
    if active {
        item = item.fill(p.accent_soft);
    }
    item.child(
        text("S")
            .font(MONO_SMALL)
            .color(if active { p.accent } else { p.faint })
            .w(21.0),
    )
    .child(
        col()
            .grow(1.0)
            .gap(1.0)
            .child(
                text("设置")
                    .font(BODY_MEDIUM)
                    .color(if active { p.text } else { p.muted }),
            )
            .child(text("出口 IP 与自动检测").font(CAPTION).color(if active {
                p.muted
            } else {
                p.faint
            })),
    )
}

fn step_nav_item(state: &State, p: Paint, step: Step) -> El {
    let active = state.step == step;
    let errors = state.step_issue_count(step);
    let title = i18n::tr(step.title());
    let marker_color = if active {
        p.accent
    } else if errors > 0 {
        p.danger
    } else {
        p.faint
    };
    let mut item = row()
        .key(format!("step-{}", step.index()))
        .role("button")
        .label(title.clone())
        .focusable(true)
        .w_full()
        .h(48.0)
        .px(10.0)
        .gap(10.0)
        .center_y()
        .radius(9.0)
        .hover(if active {
            p.accent_soft
        } else {
            Color::WHITE.alpha(0.035)
        })
        .on_press(move |s: &mut State| s.set_step(step));
    if active {
        item = item.fill(p.accent_soft);
    }
    item.child(
        text(step.number())
            .font(MONO_SMALL)
            .color(marker_color)
            .w(21.0),
    )
    .child(
        col()
            .grow(1.0)
            .gap(1.0)
            .child(text(step.title()).font(BODY_MEDIUM).color(if active {
                p.text
            } else {
                p.muted
            }))
            .child(
                text(step.short())
                    .font(CAPTION)
                    .color(if active { p.muted } else { p.faint }),
            ),
    )
    .child(if errors > 0 {
        pill(p, &errors.to_string(), p.danger)
    } else {
        el().w(3.0)
            .h(24.0)
            .radius(2.0)
            .fill(if active { p.accent } else { Color::TRANSPARENT })
    })
}

fn header(state: &State, p: Paint) -> El {
    let status = if state.step == Step::Settings {
        pill(p, "本机设置", p.info)
    } else if state.phase.is_busy() {
        pill(p, state.phase.label(), p.warning)
    } else if state.dirty {
        pill(p, "未保存", p.warning)
    } else {
        pill(p, "配置就绪", p.success)
    };
    let mut nav = row().gap(8.0);
    if let Some(previous) = state.step.previous() {
        nav = nav.child(button(
            p,
            "nav-prev",
            "上一步",
            ButtonKind::Ghost,
            move |s: &mut State| s.set_step(previous),
        ));
    }
    if let Some(next) = state.step.next() {
        nav = nav.child(button(
            p,
            "nav-next",
            "下一步",
            ButtonKind::Secondary,
            move |s: &mut State| s.set_step(next),
        ));
    }

    row()
        .w_full()
        .h(82.0)
        .px(20.0)
        .py(13.0)
        .justify_between()
        .center_y()
        .border_sides([0.0, 0.0, 1.0, 0.0], p.border)
        .child(
            col()
                .gap(4.0)
                .child(
                    row()
                        .gap(9.0)
                        .center_y()
                        .child(text("AIP // 打包工作台").font(MONO_SMALL).color(p.accent))
                        .child(dot(p.border_strong, 3.0))
                        .child(text(state.step.title()).font(MONO_SMALL).color(p.faint)),
                )
                .child(text(state.step.title()).font(DISPLAY).color(p.text))
                .child(text(state.step.description()).font(CAPTION).color(p.muted)),
        )
        .child(col().gap(10.0).items_end().child(status).child(nav))
}

fn editor(state: &State, p: Paint) -> El {
    let content = match state.step {
        Step::App => app_step(state, p),
        Step::Defaults => defaults_step(state, p),
        Step::Inputs => inputs_step(state, p),
        Step::Package => package_step(state, p),
        Step::Settings => settings_page(state, p),
    };
    col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .scroll(ScrollSpec::default())
        .child(col().w_full().padding(28.0).gap(16.0).child(content))
}

fn app_step(state: &State, p: Paint) -> El {
    let icon_error = state
        .issue(UiField::Icon)
        .map(|issue| issue.message.as_str());
    let background_error = state
        .issue(UiField::BackgroundImage)
        .or_else(|| state.issue(UiField::BackgroundColor))
        .map(|issue| issue.message.as_str());

    let mut background_modes = row().gap(6.0);
    for mode in BackgroundMode::ALL {
        let selected = state.background_mode == mode;
        background_modes = background_modes.child(segment(
            p,
            &format!("background-mode-{}", mode.label()),
            mode.label(),
            selected,
            move |s: &mut State, tx| s.set_background_mode(mode, tx),
        ));
    }

    let mut background_details = col().w_full().gap(12.0);
    match state.background_mode {
        BackgroundMode::Default => {
            background_details = background_details.child(info_box(
                p,
                "未设置 background 键。安装器使用 AIP 规定的白色回退背景。",
                p.info,
            ));
        }
        BackgroundMode::Image => {
            background_details = background_details.child(path_field(
                p,
                state,
                "bg-image",
                "背景图片",
                "绝对路径，或相对于 config.toml 的路径",
                &state.background_image_edit,
                "assets\\background.avif",
                PathTarget::Background,
                background_error,
                |s| &mut s.background_image_edit,
            ));
        }
        BackgroundMode::Color => {
            background_details = background_details.child(field(
                p,
                "bg-color",
                "十六进制颜色",
                "必须是 6 位 #RRGGBB；不接受 #RGB 或带 alpha 的颜色",
                &state.background_color_edit,
                "#1E90FF",
                background_error,
                |s| &mut s.background_color_edit,
            ));
        }
        BackgroundMode::Material => {
            let mut materials = row().gap(6.0);
            for material in BackgroundMaterial::ALL {
                materials = materials.child(segment(
                    p,
                    &format!("material-{}", material.value()),
                    material.label(),
                    state.background_material == material,
                    move |s: &mut State, tx| s.set_background_material(material, tx),
                ));
            }
            background_details = background_details
                .child(text("Windows 材质").font(MONO_SMALL).color(p.faint))
                .child(materials)
                .child(info_box(
                    p,
                    "材质不受支持时，客户端按协议回退到白色背景，不中断安装。",
                    p.info,
                ));
        }
    }

    col()
        .w_full()
        .gap(16.0)
        .child(
            card(
                p,
                "应用身份",
                "这些值会写入 config.toml 并用于窗口标题与快捷方式。",
            )
            .child(field(
                p,
                "field-name",
                "应用名称",
                "1–128 个字符，不能包含控制字符",
                &state.name_edit,
                "例如：Example App",
                state
                    .issue(UiField::Name)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.name_edit,
            ))
            .child(path_field(
                p,
                state,
                "field-icon",
                "应用图标（可选）",
                "推荐 .ico；也可由打包器转换 PNG、WebP、AVIF",
                &state.icon_edit,
                r"C:\assets\app.ico",
                PathTarget::Icon,
                icon_error,
                |s| &mut s.icon_edit,
            )),
        )
        .child(
            card(
                p,
                "安装界面背景",
                "支持图片、特殊材质或纯色。背景只影响安装界面。",
            )
            .child(background_modes)
            .child(background_details),
        )
}

fn defaults_step(state: &State, p: Paint) -> El {
    col()
        .w_full()
        .gap(16.0)
        .child(
            card(
                p,
                "默认安装位置",
                "client 用它初始化输入框，backend 用它初始化会话状态。",
            )
            .child(field(
                p,
                "field-install-location",
                "installLocation",
                "必须是本地绝对盘符路径；支持 %VAR% 环境变量",
                &state.install_location_edit,
                r"%LocalAppData%\Programs\Example App",
                state
                    .issue(UiField::InstallLocation)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.install_location_edit,
            ))
            .child(
                row()
                    .gap(8.0)
                    .child(button_tx(
                        p,
                        "suggest-install-path",
                        "使用推荐路径",
                        ButtonKind::Ghost,
                        |s: &mut State, tx| s.use_recommended_install_location(tx),
                    ))
                    .child(
                        text("推荐值不会随名称自动变化，避免覆盖手工修改。")
                            .font(CAPTION)
                            .color(p.faint),
                    ),
            ),
        )
        .child(
            card(
                p,
                "快捷方式",
                "两个选项都是安装器的初始值，用户可在安装时修改。",
            )
            .child(switch_row(
                p,
                "toggle-start-menu",
                "创建开始菜单文件夹",
                "安装到当前用户的 FOLDERID_Programs。",
                state.create_start_menu_folder,
                BoolField::StartMenu,
            ))
            .child(divider(p))
            .child(switch_row(
                p,
                "toggle-desktop",
                "创建桌面快捷方式",
                "安装到当前用户的 FOLDERID_Desktop。",
                state.create_desktop_shortcut,
                BoolField::Desktop,
            )),
        )
        .child(
            card(
                p,
                "AIP 安全边界",
                "默认值不是任意脚本入口，安装行为仍由 backend 固定执行。",
            )
            .child(info_box(
                p,
                "安装位置会由 backend 展开环境变量、检查受保护目录与 reparse point。",
                p.info,
            ))
            .child(info_box(
                p,
                "payload 中的程序、脚本与 DLL 不会被安装器执行。",
                p.success,
            )),
        )
}

fn market_components_card(state: &State, p: Paint) -> El {
    let mut components = col().w_full().gap(10.0);
    let mut contribute = String::new();
    let mut catalog_title = "组件市场".to_string();
    let mut catalog_subtitle =
        "从 jsDelivr 读取目录，从 GitHub Release 下载，并校验 SHA-256。".to_string();
    if let Some(catalog) = &state.catalog {
        contribute = catalog.marketplace.contribute.clone();
        catalog_title = if catalog.marketplace.name == "Anyinstaller Client & Backend Marketplace" {
            i18n::tr("Anyinstaller 客户端与后端市场")
        } else {
            i18n::tr(&catalog.marketplace.name)
        };
        if !catalog.marketplace.description.is_empty() {
            let description = if catalog.marketplace.description
                == "AIP v1 clients, backends, and stubs distributed through GitHub Releases."
            {
                i18n::tr("通过 GitHub Release 分发的 AIP v1 客户端、后端与 stub。")
            } else {
                i18n::tr(&catalog.marketplace.description)
            };
            catalog_subtitle = format!(
                "{}{}",
                description,
                if catalog.updated.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", catalog.updated)
                }
            );
        }
        for component in &catalog.components {
            components = components.child(component_card(
                state,
                p,
                component,
                state.market_download.as_ref(),
            ));
        }
    } else {
        components = components.child(info_box(
            p,
            if state.catalog_loading {
                "正在通过 jsDelivr 获取组件目录..."
            } else {
                "尚未载入组件目录。"
            },
            if state.catalog_loading {
                p.info
            } else {
                p.warning
            },
        ));
    }

    let mut section = card(p, &catalog_title, &catalog_subtitle).child(
        row()
            .w_full()
            .justify_between()
            .center_y()
            .gap(10.0)
            .child(
                text("选择市场组件后会自动填入下方运行组件路径。")
                    .font(CAPTION)
                    .color(p.faint),
            )
            .child(button_tx(
                p,
                "market-refresh",
                if state.catalog_loading {
                    "刷新中…"
                } else {
                    "刷新市场"
                },
                ButtonKind::Secondary,
                |s: &mut State, _tx| s.refresh_catalog(),
            )),
    );
    if let Some(error) = &state.catalog_error {
        section = section.child(warning_box(p, error, p.danger));
    }
    section = section.child(components);
    if !contribute.is_empty() {
        let url = contribute;
        section = section.child(
            row()
                .w_full()
                .justify_between()
                .center_y()
                .child(
                    text("组件作者可以通过 PR 提交 Release 条目。")
                        .font(CAPTION)
                        .color(p.faint),
                )
                .child(button_tx(
                    p,
                    "market-contribute",
                    "贡献说明",
                    ButtonKind::Ghost,
                    move |_: &mut State, tx| {
                        let _ = tx.open_url(&url);
                    },
                )),
        );
    }
    section
}

fn language_card(state: &State, p: Paint) -> El {
    let mut rows = col().w_full().gap(7.0);
    for chunk in Language::ALL.chunks(3) {
        let mut items = row().w_full().gap(7.0);
        for language in chunk {
            items = items.child(segment_tx(
                p,
                &format!("language-{}", language.code()),
                language.native_name(),
                state.language == *language,
                move |s: &mut State, tx| s.set_language(*language, tx),
            ));
        }
        rows = rows.child(items);
    }
    card(p, "界面语言", "语言切换会立即生效，并保存在本机设置中。").child(rows)
}

fn settings_page(state: &State, p: Paint) -> El {
    let (region_color, region_detail) = match &state.region_status {
        RegionStatus::Domestic { fallback, .. } => (
            p.success,
            if *fallback {
                "ipwho.is 不可用 · 默认国内"
            } else {
                "国内 · 自动开启"
            },
        ),
        RegionStatus::Overseas { .. } => (p.info, "海外 · 自动关闭"),
        RegionStatus::Checking => (p.warning, "正在检测"),
        RegionStatus::Unknown => (p.faint, "等待自动检测"),
    };
    let market_status = if state.catalog_loading {
        "市场目录正在加载"
    } else if state.catalog.is_some() {
        "市场目录已加载"
    } else if state.catalog_error.is_some() {
        "市场目录加载失败"
    } else {
        "市场目录尚未加载"
    };

    let network_card = card(
        p,
        "出口 IP 与下载线路",
        "自动检测当前出口 IP；国内开启 GitHub 加速，海外自动关闭；ipwho.is 不可用时默认按国内处理。",
    )
    .child(
        row()
            .w_full()
            .gap(10.0)
            .center_y()
            .child(dot(region_color, 8.0))
            .child(
                text(state.region_status.label())
                    .font(BODY_MEDIUM)
                    .color(p.text)
                    .grow(1.0)
                    .truncate(),
            )
            .child(pill(p, region_detail, region_color)),
    )
    .child(setting_path_row(p, "当前出口 IP", &state.region_status.ip()))
    .child(
        row()
            .w_full()
            .gap(8.0)
            .child(button_tx(
                p,
                "settings-detect-region",
                "重新检测出口 IP",
                ButtonKind::Secondary,
                |s: &mut State, _tx| s.check_region(),
            ))
            .child(
                text("仅访问 ipwho.is，不发送账号、配置或本机文件。")
                    .font(CAPTION)
                    .color(p.faint),
            ),
    )
    .child(setting_field(
        p,
        "settings-accelerator-prefix",
        "下载加速前缀",
        &state.accelerator_prefix_edit,
        state.accelerator_error.as_deref(),
        |s| &mut s.accelerator_prefix_edit,
    ))
    .child(switch_row(
        p,
        "settings-toggle-acceleration",
        "使用 GitHub 下载加速",
        "国内自动开启，海外自动关闭；仍可手动覆盖。下载失败时回退 GitHub 直连。",
        state.accelerator_enabled,
        BoolField::Acceleration,
    ))
    .child(
        row()
            .gap(8.0)
            .child(button_tx(
                p,
                "settings-save-acceleration",
                "保存网络设置",
                ButtonKind::Ghost,
                |s: &mut State, tx| s.save_acceleration(tx),
            ))
            .child(button_tx(
                p,
                "settings-auto-detect",
                "立即自动检测",
                ButtonKind::Primary,
                |s: &mut State, tx| s.run_auto_detection(tx),
            )),
    );
    let settings_path = crate::settings::settings_path();
    let cache_path = market::cache_root();
    col()
        .w_full()
        .gap(16.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .center_y()
                .child(
                    col()
                        .gap(2.0)
                        .child(text("偏好设置").font(MONO_SMALL).color(p.accent))
                        .child(
                            text(state.autodetect_note.clone())
                                .font(CAPTION)
                                .color(p.muted),
                        ),
                )
                .child(button(
                    p,
                    "settings-back-inputs",
                    "返回运行组件",
                    ButtonKind::Ghost,
                    |s: &mut State| s.set_step(Step::Inputs),
                )),
        )
        .child(language_card(state, p))
        .child(network_card)
        .child(
            card(
                p,
                "工作区自动定位",
                "自动寻找 packager、stub、client、backend 与客户端 assets。",
            )
            .child(info_box(p, &state.autodetect_note, p.info))
            .child(
                row()
                    .gap(8.0)
                    .child(button_tx(
                        p,
                        "settings-detect-workspace",
                        "重新自动定位组件",
                        ButtonKind::Secondary,
                        |s: &mut State, tx| s.auto_detect(Some(tx)),
                    ))
                    .child(
                        text("检测结果只填充输入路径，之后仍可逐个浏览并改成本机文件。")
                            .font(CAPTION)
                            .color(p.faint),
                    ),
            ),
        )
        .child(
            card(
                p,
                "本机数据",
                "设置和组件缓存位于用户数据目录，不写入 AIP 包。",
            )
            .child(setting_path_row(
                p,
                "设置文件",
                &settings_path.display().to_string(),
            ))
            .child(divider(p))
            .child(setting_path_row(
                p,
                "组件缓存",
                &cache_path.display().to_string(),
            ))
            .child(divider(p))
            .child(setting_path_row(p, "市场目录", market::CATALOG_URL))
            .child(info_box(p, market_status, p.muted)),
        )
}

fn setting_path_row(p: Paint, label: &str, value: &str) -> El {
    row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(text(label).font(LABEL).color(p.text).w(76.0))
        .child(
            text(value.to_string())
                .font(MONO_SMALL)
                .color(p.muted)
                .grow(1.0)
                .truncate(),
        )
}

fn component_card(
    state: &State,
    p: Paint,
    component: &crate::market::Component,
    active_download: Option<&MarketDownload>,
) -> El {
    let installed = state.installed_component(&component.id);
    let is_downloading =
        active_download.is_some_and(|download| download.component_id == component.id);
    let other_downloading = active_download.is_some() && !is_downloading;
    let kind_color = match component.kind {
        ComponentKind::Client => p.info,
        ComponentKind::Backend => p.success,
        ComponentKind::Stub => p.warning,
    };
    let homepage = component.homepage.clone();
    let download_component = component.clone();
    let source = if component.source_repository.is_empty() {
        component.repository.clone()
    } else {
        format!(
            "{}  ·  {} {}",
            component.repository,
            i18n::tr("源码"),
            component.source_repository
        )
    };

    let mut actions = row().gap(8.0);
    if !homepage.is_empty() {
        actions = actions.child(button_tx(
            p,
            &format!("component-home-{}", component.id),
            "项目主页",
            ButtonKind::Ghost,
            move |_: &mut State, tx| {
                let _ = tx.open_url(&homepage);
            },
        ));
    }
    if installed.is_some() {
        let id = component.id.clone();
        actions = actions.child(button_tx(
            p,
            &format!("component-use-{}", component.id),
            "使用缓存",
            ButtonKind::Secondary,
            move |s: &mut State, tx| s.use_installed_component(&id, tx),
        ));
        actions = actions.child(pill(p, "已安装", p.success));
    }
    let download_button = button_tx(
        p,
        &format!("component-download-{}", component.id),
        if is_downloading {
            "下载中…"
        } else {
            "下载并使用"
        },
        ButtonKind::Primary,
        move |s: &mut State, tx| s.start_market_download(download_component.clone(), tx),
    );
    actions = actions.child(if is_downloading || other_downloading {
        download_button.disabled(true)
    } else {
        download_button
    });

    let mut card_body = col()
        .w_full()
        .gap(7.0)
        .padding(10.0)
        .radius(8.0)
        .fill(p.surface_high)
        .border(1.0, p.border)
        .help(i18n::tr(&component.description))
        .child(
            row()
                .w_full()
                .gap(7.0)
                .center_y()
                .child(
                    text(component.name.clone())
                        .font(BODY_MEDIUM)
                        .color(p.text)
                        .grow(1.0)
                        .truncate(),
                )
                .child(pill(p, &format!("v{}", component.version), p.info))
                .child(pill(p, component.kind.label(), kind_color))
                .child(pill(p, &component.license, p.success))
                .child(if component.official {
                    pill(p, "官方", p.accent)
                } else {
                    pill(p, "社区", p.muted)
                }),
        )
        .child(
            row()
                .w_full()
                .justify_between()
                .gap(10.0)
                .center_y()
                .child(
                    text(source)
                        .font(CAPTION)
                        .color(p.faint)
                        .grow(1.0)
                        .truncate(),
                )
                .child(actions),
        );
    if let Some(download) = active_download.filter(|download| download.component_id == component.id)
    {
        card_body = card_body.child(download_progress(p, download));
    }
    card_body
}

fn download_progress(p: Paint, download: &MarketDownload) -> El {
    let fraction = download
        .total
        .filter(|total| *total > 0)
        .map(|total| (download.received as f32 / total as f32).clamp(0.0, 1.0))
        .unwrap_or(0.12);
    let bytes = match download.total {
        Some(total) => format!(
            "{} / {}",
            human_bytes(download.received),
            human_bytes(total)
        ),
        None => human_bytes(download.received),
    };
    col()
        .w_full()
        .gap(7.0)
        .padding(11.0)
        .radius(8.0)
        .fill(p.field)
        .border(1.0, p.border)
        .child(
            row()
                .justify_between()
                .child(text(download.message.clone()).font(CAPTION).color(p.info))
                .child(text(bytes).font(MONO_SMALL).color(p.muted)),
        )
        .child(
            el().w_full()
                .h(5.0)
                .radius(3.0)
                .fill(p.surface)
                .child(el().w_frac(fraction).h(5.0).radius(3.0).fill(p.accent)),
        )
}

fn setting_field(
    p: Paint,
    key: &str,
    label: &str,
    edit: &rune::TextEdit,
    error: Option<&str>,
    accessor: fn(&mut State) -> &mut rune::TextEdit,
) -> El {
    let border = if error.is_some() {
        p.danger
    } else {
        p.border_strong
    };
    let label = i18n::tr(label);
    col()
        .w_full()
        .gap(7.0)
        .child(
            row()
                .justify_between()
                .center_y()
                .child(text(label.clone()).font(LABEL).color(p.text))
                .child(if let Some(error) = error {
                    text(error).font(CAPTION).color(p.danger).truncate()
                } else {
                    text("例如 https://gh.meali.top/")
                        .font(CAPTION)
                        .color(p.faint)
                }),
        )
        .child(
            text_input(edit, BODY)
                .key(key)
                .label(label.clone())
                .single_line()
                .w_full()
                .h(36.0)
                .px(10.0)
                .py(8.0)
                .radius(8.0)
                .fill(p.field)
                .border(1.0, border)
                .focus_ring(2.0, 1.0, p.accent)
                .color(p.text)
                .placeholder(market::DEFAULT_GITHUB_ACCELERATOR, p.faint)
                .on_edit(accessor),
        )
}

fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{:.0} B", bytes)
    }
}

fn inputs_step(state: &State, p: Paint) -> El {
    let auto_note = state.autodetect_note.clone();
    let mut card_exec = card(
        p,
        "运行组件",
        "界面会把 stub、client、backend 同步为构建目录中的固定文件名。",
    );
    card_exec = card_exec
        .child(path_field(
            p,
            state,
            "field-stub",
            "stub.exe",
            "必须是不带现有 Authenticode 证书表的未签名 stub",
            &state.stub_edit,
            r"stub\target\release\stub.exe",
            PathTarget::Stub,
            state
                .issue(UiField::Stub)
                .map(|issue| issue.message.as_str()),
            |s| &mut s.stub_edit,
        ))
        .child(divider(p))
        .child(path_field(
            p,
            state,
            "field-client",
            "client.exe",
            "AIP 安装界面进程",
            &state.client_edit,
            r"voidclient\dist\client\client.exe",
            PathTarget::Client,
            state
                .issue(UiField::Client)
                .map(|issue| issue.message.as_str()),
            |s| &mut s.client_edit,
        ))
        .child(divider(p))
        .child(path_field(
            p,
            state,
            "field-backend",
            "backend.exe",
            "AIP 安装执行器",
            &state.backend_edit,
            r"voidbackend\target\release\backend.exe",
            PathTarget::Backend,
            state
                .issue(UiField::Backend)
                .map(|issue| issue.message.as_str()),
            |s| &mut s.backend_edit,
        ));

    col()
        .w_full()
        .gap(16.0)
        .child(
            row()
                .w_full()
                .justify_between()
                .center_y()
                .child(
                    col()
                        .gap(2.0)
                        .child(text("自动定位").font(MONO_SMALL).color(p.accent))
                        .child(text(auto_note).font(CAPTION).color(p.muted)),
                )
                .child(
                    row()
                        .gap(7.0)
                        .child(button_tx(
                            p,
                            "auto-detect",
                            "重新定位",
                            ButtonKind::Secondary,
                            |s: &mut State, tx| s.auto_detect(Some(tx)),
                        ))
                        .child(button_tx(
                            p,
                            "import-custom-component",
                            "导入自定义",
                            ButtonKind::Secondary,
                            |s: &mut State, tx| s.browse(tx, PathTarget::ImportCustomComponent),
                        ))
                        .child(button_tx(
                            p,
                            "inputs-open-settings",
                            "网络设置",
                            ButtonKind::Ghost,
                            |s: &mut State, _tx| s.open_settings(),
                        )),
                ),
        )
        .child(card_exec)
        .child(market_components_card(state, p))
        .child(
            card(
                p,
                "载荷与客户端资源",
                "payload.zip 是安装载荷；客户端 assets 会递归合并到 build/assets。",
            )
            .child(path_field(
                p,
                state,
                "field-payload",
                "payload.zip",
                "ZIP 至少包含一个文件，只允许 store/deflate",
                &state.payload_edit,
                r"payload.zip",
                PathTarget::Payload,
                state
                    .issue(UiField::Payload)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.payload_edit,
            ))
            .child(divider(p))
            .child(path_field(
                p,
                state,
                "field-client-assets",
                "客户端资源目录（assets，可选）",
                "PyInstaller 目录式构建的 assets 目录，内容会合并进构建资源",
                &state.client_assets_edit,
                r"voidclient\dist\client\assets",
                PathTarget::ClientAssets,
                state
                    .issue(UiField::ClientAssets)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.client_assets_edit,
            )),
        )
        .child(
            card(
                p,
                "构建目录",
                "config.toml 与固定命名的输入文件会写入这里；输出必须在目录外。",
            )
            .child(path_field(
                p,
                state,
                "field-build-dir",
                "构建目录",
                "目录不存在时会自动创建",
                &state.build_dir_edit,
                r"target\gui-build",
                PathTarget::BuildDir,
                state
                    .issue(UiField::BuildDir)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.build_dir_edit,
            )),
        )
        .child(info_box(
            p,
            "同步是合并写入，不会在打包前删除构建目录中已有的资源文件。",
            p.warning,
        ))
}

fn package_step(state: &State, p: Paint) -> El {
    let packager_error = state
        .issue(UiField::Packager)
        .map(|issue| issue.message.as_str());
    let output_error = state
        .issue(UiField::Output)
        .map(|issue| issue.message.as_str());

    let mut sign_modes = row().gap(6.0);
    for mode in SignMode::ALL {
        sign_modes = sign_modes.child(segment_tx(
            p,
            &format!("sign-mode-{}", mode.label()),
            mode.label(),
            state.sign_mode == mode,
            move |s: &mut State, tx| {
                if s.sign_mode != mode {
                    s.sign_mode = mode;
                    s.mark_dirty(tx);
                }
            },
        ));
    }

    let mut signing_card = card(
        p,
        "签名方式",
        "正式发布建议配置代码签名和 RFC 3161 时间戳。",
    )
    .child(sign_modes)
    .child(
        text(state.sign_mode.description())
            .font(CAPTION)
            .color(p.muted),
    );
    match state.sign_mode {
        SignMode::Unsigned => {
            signing_card = signing_card.child(warning_box(
                p,
                "未配置签名：正式发布不推荐使用。",
                p.warning,
            ));
        }
        SignMode::Thumbprint => {
            signing_card = signing_card.child(field(
                p,
                "field-thumbprint",
                "证书 SHA-1 指纹",
                "当前用户证书存储中的 40 位十六进制指纹",
                &state.thumbprint_edit,
                "0123456789ABCDEF0123456789ABCDEF01234567",
                state
                    .issue(UiField::Thumbprint)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.thumbprint_edit,
            ));
        }
        SignMode::Pfx => {
            signing_card = signing_card
                .child(path_field(
                    p,
                    state,
                    "field-pfx",
                    "PFX 证书",
                    "证书文件不能位于构建目录或安装目录内",
                    &state.pfx_edit,
                    r"D:\keys\release.pfx",
                    PathTarget::Pfx,
                    state
                        .issue(UiField::Pfx)
                        .map(|issue| issue.message.as_str()),
                    |s| &mut s.pfx_edit,
                ))
                .child(field(
                    p,
                    "field-pfx-env",
                    "密码环境变量",
                    "只传环境变量名，GUI 与命令行日志不会显示密码值",
                    &state.pfx_password_env_edit,
                    "AIP_PFX_PASSWORD",
                    state
                        .issue(UiField::PfxPasswordEnv)
                        .map(|issue| issue.message.as_str()),
                    |s| &mut s.pfx_password_env_edit,
                ));
        }
    }

    col()
        .w_full()
        .gap(16.0)
        .child(
            card(
                p,
                "打包器与输出",
                "打包器负责最终 AIP 校验、签名和自解包验证。",
            )
            .child(path_field(
                p,
                state,
                "field-packager",
                "packager.exe",
                "默认查找同工作区的 target/release 或与 GUI 同目录",
                &state.packager_edit,
                r"packager\target\release\packager.exe",
                PathTarget::Packager,
                packager_error,
                |s| &mut s.packager_edit,
            ))
            .child(divider(p))
            .child(path_field(
                p,
                state,
                "field-output",
                "输出安装器",
                "必须是构建目录之外的 .exe 路径；父目录会自动创建",
                &state.output_edit,
                r"target\packager-output\Example App Setup.exe",
                PathTarget::Output,
                output_error,
                |s| &mut s.output_edit,
            ))
            .child(switch_row(
                p,
                "toggle-force",
                "覆盖已有输出",
                "同名文件存在时允许打包器原子替换。",
                state.force_overwrite,
                BoolField::Force,
            )),
        )
        .child(signing_card)
        .child(
            card(
                p,
                "签名细节",
                "未填写 signtool 时，会从 PATH 与 Windows SDK 查找。",
            )
            .child(field(
                p,
                "field-timestamp",
                "RFC 3161 时间戳 URL",
                "未配置签名时不会使用此值",
                &state.timestamp_url_edit,
                DEFAULT_TIMESTAMP_URL,
                state
                    .issue(UiField::TimestampUrl)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.timestamp_url_edit,
            ))
            .child(path_field(
                p,
                state,
                "field-signtool",
                "signtool.exe（可选）",
                "留空则自动查找",
                &state.signtool_edit,
                r"C:\Program Files (x86)\Windows Kits\10\bin\...\signtool.exe",
                PathTarget::Signtool,
                state
                    .issue(UiField::Signtool)
                    .map(|issue| issue.message.as_str()),
                |s| &mut s.signtool_edit,
            ))
            .child(field(
                p,
                "field-description",
                "签名描述（可选）",
                "留空时使用应用名称",
                &state.description_edit,
                "Anyinstaller package for Example App",
                None,
                |s| &mut s.description_edit,
            )),
        )
}

fn inspector(state: &State, p: Paint) -> El {
    let input_count = configured_inputs(state);
    let status_color = match state.phase {
        BuildPhase::Idle => p.faint,
        BuildPhase::Preparing | BuildPhase::Running => p.warning,
        BuildPhase::Success => p.success,
        BuildPhase::Failure => p.danger,
    };
    let output = state
        .last_output
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| {
            let path = state.output_edit.text.trim();
            if path.is_empty() {
                "尚未设置输出".into()
            } else {
                path.to_string()
            }
        });

    let mut logs = col().w_full().gap(4.0).padding(11.0);
    for line in &state.logs {
        let color = match line.kind {
            LogKind::Info => p.info,
            LogKind::Tool => p.muted,
            LogKind::Error => p.danger,
        };
        let prefix = match line.kind {
            LogKind::Info => "›",
            LogKind::Tool => "·",
            LogKind::Error => "!",
        };
        logs = logs.child(
            text(format!("{prefix} {}", i18n::tr(&line.text)))
                .font(MONO_SMALL)
                .color(color)
                .wrap(),
        );
    }

    let progress = if state.phase.is_busy() {
        let phase = (state.spinner_time * 1.3).rem_euclid(1.0) as f32;
        el().w_full().h(3.0).radius(2.0).fill(p.surface_high).child(
            el().absolute(phase * 245.0, 0.0)
                .w(48.0)
                .h(3.0)
                .radius(2.0)
                .fill(p.accent),
        )
    } else {
        el().w_full().h(3.0).radius(2.0).fill(p.border)
    };

    col()
        .w(304.0)
        .h_full()
        .px(16.0)
        .py(16.0)
        .gap(10.0)
        .fill(p.sidebar)
        .border_sides([1.0, 0.0, 0.0, 0.0], p.border)
        .child(
            row()
                .justify_between()
                .center_y()
                .child(text("构建状态").font(MONO_SMALL).color(p.accent))
                .child(text("实时").font(MONO_SMALL).color(p.faint)),
        )
        .child(
            col()
                .w_full()
                .gap(11.0)
                .padding(14.0)
                .radius(11.0)
                .fill(p.surface)
                .border(1.0, p.border)
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(dot(status_color, 8.0))
                        .child(text(state.phase.label()).font(BODY_MEDIUM).color(p.text))
                        .child(spacer().grow(1.0))
                        .child(if state.phase.is_busy() {
                            text(format!(
                                "{:.0}%",
                                ((state.spinner_time * 0.65).sin().abs()) * 82.0 + 8.0
                            ))
                            .font(MONO)
                            .color(p.warning)
                        } else {
                            text(match state.phase {
                                BuildPhase::Success => "成功",
                                BuildPhase::Failure => "失败",
                                _ => "空闲",
                            })
                            .font(MONO)
                            .color(status_color)
                        }),
                )
                .child(progress)
                .child(metric(p, "输入", format!("{input_count} / 4")))
                .child(metric(
                    p,
                    "校验",
                    format!("{} {}", state.issues.len(), i18n::tr("问题")),
                ))
                .child(text(output).font(MONO_SMALL).color(p.muted).wrap()),
        )
        .child(
            row()
                .justify_between()
                .center_y()
                .child(text("打包日志").font(MONO_SMALL).color(p.faint))
                .child(
                    text(format!("{} {}", state.logs.len(), i18n::tr("行")))
                        .font(MONO_SMALL)
                        .color(p.faint),
                ),
        )
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .radius(9.0)
                .clip_rounded(9.0)
                .fill(p.log_bg)
                .border(1.0, p.border)
                .scroll(ScrollSpec::stick_bottom())
                .child(logs),
        )
}

fn footer(state: &State, p: Paint) -> El {
    if state.step == Step::Settings {
        return settings_footer(state, p);
    }
    let (notice_text, notice_color) = state
        .notice
        .as_ref()
        .map(|notice| {
            let color = match notice.tone {
                NoticeTone::Info => p.info,
                NoticeTone::Success => p.success,
                NoticeTone::Warning => p.warning,
                NoticeTone::Error => p.danger,
            };
            (notice.text.clone(), color)
        })
        .unwrap_or_else(|| ("准备就绪".into(), p.muted));
    let busy = state.phase.is_busy();

    let mut actions = row().gap(8.0).center_y();
    actions = actions
        .child(button_tx(
            p,
            "load-config",
            "载入配置",
            ButtonKind::Ghost,
            |s: &mut State, tx| s.browse(tx, PathTarget::LoadConfig),
        ))
        .child(button_tx(
            p,
            "save-config",
            "保存配置",
            ButtonKind::Ghost,
            |s: &mut State, tx| s.save_config(tx),
        ))
        .child(button_tx(
            p,
            "validate-config",
            "验证",
            ButtonKind::Secondary,
            |s: &mut State, _tx| {
                s.validate_config_only();
            },
        ));
    let build = button_tx(
        p,
        "start-build",
        if busy {
            "正在打包…"
        } else {
            "开始打包"
        },
        ButtonKind::Primary,
        |s: &mut State, tx| s.start_build(tx),
    );
    let build = if busy { build.disabled(true) } else { build };
    actions = actions.child(build);

    row()
        .w_full()
        .h(60.0)
        .px(20.0)
        .gap(18.0)
        .justify_between()
        .center_y()
        .border_sides([1.0, 0.0, 0.0, 0.0], p.border)
        .child(
            row()
                .grow(1.0)
                .min_w(0.0)
                .gap(9.0)
                .center_y()
                .child(dot(notice_color, 7.0))
                .child(text(notice_text).font(CAPTION).color(p.muted).truncate()),
        )
        .child(actions)
}

fn settings_footer(state: &State, p: Paint) -> El {
    let notice = state
        .notice
        .as_ref()
        .map(|notice| notice.text.clone())
        .unwrap_or_else(|| "设置保存在本机，不影响 AIP config.toml。".into());
    row()
        .w_full()
        .h(66.0)
        .px(24.0)
        .gap(18.0)
        .justify_between()
        .center_y()
        .border_sides([1.0, 0.0, 0.0, 0.0], p.border)
        .child(
            row()
                .grow(1.0)
                .min_w(0.0)
                .gap(9.0)
                .center_y()
                .child(dot(p.info, 7.0))
                .child(text(notice).font(CAPTION).color(p.muted).truncate()),
        )
        .child(
            row()
                .gap(8.0)
                .child(button_tx(
                    p,
                    "settings-footer-back",
                    "返回运行组件",
                    ButtonKind::Ghost,
                    |s: &mut State, _tx| s.set_step(Step::Inputs),
                ))
                .child(button_tx(
                    p,
                    "settings-footer-save",
                    "保存网络设置",
                    ButtonKind::Secondary,
                    |s: &mut State, tx| s.save_acceleration(tx),
                ))
                .child(button_tx(
                    p,
                    "settings-footer-detect",
                    "立即自动检测",
                    ButtonKind::Primary,
                    |s: &mut State, tx| s.run_auto_detection(tx),
                )),
        )
}

fn card(p: Paint, title: &str, subtitle: &str) -> El {
    col()
        .w_full()
        .gap(12.0)
        .padding(16.0)
        .radius(10.0)
        .fill(p.surface)
        .border(1.0, p.border)
        .child(
            col()
                .gap(3.0)
                .child(text(title).font(CARD_TITLE).color(p.text))
                .child(text(subtitle).font(CAPTION).color(p.muted).wrap()),
        )
}

#[allow(clippy::too_many_arguments)]
fn field(
    p: Paint,
    key: &str,
    label: &str,
    hint: &str,
    edit: &rune::TextEdit,
    placeholder: &str,
    error: Option<&str>,
    accessor: fn(&mut State) -> &mut rune::TextEdit,
) -> El {
    let border = if error.is_some() {
        p.danger
    } else {
        p.border_strong
    };
    let label = i18n::tr(label);
    let hint = i18n::tr(hint);
    let placeholder = i18n::tr(placeholder);
    col()
        .w_full()
        .gap(7.0)
        .child(
            row()
                .justify_between()
                .center_y()
                .child(text(label.clone()).font(LABEL).color(p.text))
                .child(if let Some(error) = error {
                    text(error).font(CAPTION).color(p.danger).truncate()
                } else {
                    text(hint).font(CAPTION).color(p.faint).truncate()
                }),
        )
        .child(
            text_input(edit, BODY)
                .key(key)
                .label(label.clone())
                .single_line()
                .w_full()
                .h(40.0)
                .px(12.0)
                .py(10.0)
                .radius(8.0)
                .fill(p.field)
                .border(1.0, border)
                .focus_ring(2.0, 1.0, p.accent)
                .color(p.text)
                .placeholder(placeholder, p.faint)
                .on_edit(accessor)
                .on_input_change(|s: &mut State, tx| s.mark_dirty(tx)),
        )
}

#[allow(clippy::too_many_arguments)]
fn path_field(
    p: Paint,
    _state: &State,
    key: &str,
    label: &str,
    hint: &str,
    edit: &rune::TextEdit,
    placeholder: &str,
    target: PathTarget,
    error: Option<&str>,
    accessor: fn(&mut State) -> &mut rune::TextEdit,
) -> El {
    let border = if error.is_some() {
        p.danger
    } else {
        p.border_strong
    };
    let label = i18n::tr(label);
    let hint = i18n::tr(hint);
    let placeholder = i18n::tr(placeholder);
    let exists = path_exists(edit.text.trim());
    let status = if edit.text.trim().is_empty() {
        pill(p, "待选择", p.faint)
    } else if exists {
        pill(p, "已找到", p.success)
    } else if target == PathTarget::BuildDir {
        pill(p, "将创建", p.info)
    } else if target == PathTarget::Output {
        pill(p, "待生成", p.info)
    } else {
        pill(p, "未找到", p.warning)
    };
    let input = text_input(edit, BODY)
        .key(key)
        .label(label.clone())
        .single_line()
        .grow(1.0)
        .min_w(0.0)
        .h(36.0)
        .px(10.0)
        .py(8.0)
        .radius(8.0)
        .fill(p.field)
        .border(1.0, border)
        .focus_ring(2.0, 1.0, p.accent)
        .color(p.text)
        .placeholder(placeholder, p.faint)
        .on_edit(accessor)
        .on_input_change(|s: &mut State, tx| s.mark_dirty(tx));

    col()
        .w_full()
        .gap(7.0)
        .child(
            row()
                .justify_between()
                .center_y()
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(text(label.clone()).font(LABEL).color(p.text))
                        .child(status),
                )
                .child(if let Some(error) = error {
                    text(error).font(CAPTION).color(p.danger).truncate()
                } else {
                    text(hint).font(CAPTION).color(p.faint).truncate()
                }),
        )
        .child(
            row()
                .w_full()
                .gap(7.0)
                .center_y()
                .child(input)
                .child(button_tx(
                    p,
                    &format!("{key}-browse"),
                    "浏览",
                    ButtonKind::Secondary,
                    move |s: &mut State, tx| s.browse(tx, target),
                )),
        )
}

fn segment(
    p: Paint,
    key: &str,
    label: &str,
    selected: bool,
    action: impl Fn(&mut State, &mut rune::Tx) + 'static,
) -> El {
    segment_tx(p, key, label, selected, action)
}

fn segment_tx(
    p: Paint,
    key: &str,
    label: &str,
    selected: bool,
    action: impl Fn(&mut State, &mut rune::Tx) + 'static,
) -> El {
    let localized = i18n::tr(label);
    el().key(key)
        .role("segment")
        .label(localized.clone())
        .checked(selected)
        .focusable(true)
        .h(32.0)
        .px(12.0)
        .center()
        .radius(7.0)
        .fill(if selected { p.accent_soft } else { p.field })
        .border(
            1.0,
            if selected {
                p.accent.alpha(0.45)
            } else {
                p.border
            },
        )
        .hover(if selected {
            Color::TRANSPARENT
        } else {
            Color::WHITE.alpha(0.04)
        })
        .on_press_tx(action)
        .child(
            text(localized)
                .font(BUTTON)
                .color(if selected { p.accent } else { p.muted })
                .a11y_hidden(),
        )
}

fn switch_row(
    p: Paint,
    key: &str,
    label: &str,
    description: &str,
    on: bool,
    field: BoolField,
) -> El {
    let control = el()
        .key("switch")
        .size(40.0, 23.0)
        .radius(12.0)
        .motion(motion::SNAPPY)
        .fill(if on { p.accent } else { p.surface_high })
        .child(
            el().key("knob")
                .size(17.0, 17.0)
                .absolute(if on { 20.0 } else { 3.0 }, 3.0)
                .radius(9.0)
                .fill(Color::WHITE)
                .motion(motion::SNAPPY),
        )
        .a11y_hidden();
    let localized = i18n::tr(label);
    row()
        .key(key)
        .role("switch")
        .label(localized.clone())
        .checked(on)
        .focusable(true)
        .w_full()
        .min_h(50.0)
        .py(8.0)
        .gap(16.0)
        .center_y()
        .child(
            col()
                .grow(1.0)
                .gap(2.0)
                .child(text(localized).font(BODY_MEDIUM).color(p.text))
                .child(text(description).font(CAPTION).color(p.muted).wrap()),
        )
        .child(control)
        .on_press_tx(move |s: &mut State, tx| match field {
            BoolField::StartMenu => {
                s.create_start_menu_folder = !s.create_start_menu_folder;
                s.mark_dirty(tx);
            }
            BoolField::Desktop => {
                s.create_desktop_shortcut = !s.create_desktop_shortcut;
                s.mark_dirty(tx);
            }
            BoolField::Force => {
                s.force_overwrite = !s.force_overwrite;
                s.mark_dirty(tx);
            }
            BoolField::Acceleration => s.toggle_acceleration(tx),
        })
}

fn button(
    p: Paint,
    key: &str,
    label: &str,
    kind: ButtonKind,
    action: impl Fn(&mut State) + 'static,
) -> El {
    button_base(p, key, label, kind).on_press(action)
}

fn button_tx(
    p: Paint,
    key: &str,
    label: &str,
    kind: ButtonKind,
    action: impl Fn(&mut State, &mut rune::Tx) + 'static,
) -> El {
    button_base(p, key, label, kind).on_press_tx(action)
}

fn button_base(p: Paint, key: &str, label: &str, kind: ButtonKind) -> El {
    let localized = i18n::tr(label);
    let (fill, hover, press, ink, border) = match kind {
        ButtonKind::Primary => (
            p.accent,
            p.accent_hover,
            p.accent_hover,
            Color::WHITE,
            Color::TRANSPARENT,
        ),
        ButtonKind::Secondary => (
            p.surface_high,
            Color::WHITE.alpha(0.08),
            p.field,
            p.text,
            p.border_strong,
        ),
        ButtonKind::Ghost => (
            Color::TRANSPARENT,
            Color::WHITE.alpha(0.04),
            Color::WHITE.alpha(0.06),
            p.muted,
            p.border,
        ),
    };
    el().key(key)
        .role("button")
        .label(localized.clone())
        .focusable(true)
        .h(32.0)
        .px(11.0)
        .center()
        .radius(8.0)
        .fill(fill)
        .border(1.0, border)
        .hover(hover)
        .press(press)
        .focus_ring(2.0, 1.0, p.accent)
        .child(text(localized).font(BUTTON).color(ink).a11y_hidden())
}

fn pill(p: Paint, label: &str, color: Color) -> El {
    let localized = i18n::tr(label);
    el().h(21.0)
        .px(7.0)
        .center()
        .radius(10.0)
        .fill(p.surface_high)
        .border(1.0, color.alpha(0.24))
        .child(text(localized).font(MONO_SMALL).color(color))
}

fn metric(p: Paint, label: &str, value: String) -> El {
    row()
        .w_full()
        .justify_between()
        .center_y()
        .child(text(label).font(CAPTION).color(p.faint))
        .child(text(value).font(MONO_SMALL).color(p.muted))
}

fn dot(color: Color, size: f32) -> El {
    el().size(size, size).radius(size / 2.0).fill(color)
}

fn divider(p: Paint) -> El {
    el().w_full().h(1.0).fill(p.border)
}

fn info_box(p: Paint, message: &str, color: Color) -> El {
    row()
        .w_full()
        .min_h(38.0)
        .px(11.0)
        .py(8.0)
        .gap(9.0)
        .center_y()
        .radius(8.0)
        .fill(color.alpha(0.08))
        .border(1.0, color.alpha(0.16))
        .child(dot(color, 6.0))
        .child(text(message).font(CAPTION).color(p.muted).wrap())
}

fn warning_box(p: Paint, message: &str, color: Color) -> El {
    info_box(p, message, color)
}

fn configured_inputs(state: &State) -> usize {
    [
        &state.stub_edit,
        &state.client_edit,
        &state.backend_edit,
        &state.payload_edit,
    ]
    .into_iter()
    .filter(|edit| !edit.text.trim().is_empty())
    .count()
}

fn path_exists(value: &str) -> bool {
    !value.is_empty() && std::path::Path::new(value).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn main_workflow_renders_each_step() {
        for step in Step::ALL {
            let state = State {
                step,
                ..State::default()
            };
            let mut runtime = Runtime::new(state, view, (1180.0, 760.0), Env::new(true, 1.0));
            runtime.frame(0.0);
            let key = format!("step-{}", step.index());
            assert!(runtime.find(&key).is_some());
        }
    }

    #[test]
    fn dedicated_settings_page_exposes_network_and_detection_controls() {
        let state = State {
            step: Step::Settings,
            region_status: RegionStatus::Domestic {
                ip: "198.51.100.8".into(),
                country_code: "CN".into(),
                country: "China".into(),
                fallback: false,
            },
            ..State::default()
        };
        let mut runtime = Runtime::new(state, view, (1280.0, 800.0), Env::new(true, 1.0));
        runtime.frame(0.0);
        assert!(runtime.find("settings-nav").is_some());
        assert!(runtime.find("settings-accelerator-prefix").is_some());
        assert!(runtime.find("settings-auto-detect").is_some());
        assert!(runtime.find("settings-detect-workspace").is_some());
        for code in ["en", "zh-cn", "hi", "es", "fr", "ar"] {
            assert!(runtime.find(&format!("language-{code}")).is_some());
        }
    }

    #[test]
    fn sidebar_navigation_changes_step() {
        let mut runtime =
            Runtime::new(State::default(), view, (1280.0, 800.0), Env::new(true, 1.0));
        runtime.frame(0.0);
        let (x, y) = runtime.find("step-2").unwrap();
        runtime.pointer_down(x, y);
        runtime.pointer_up(x, y);
        runtime.frame(0.0);
        assert_eq!(runtime.state.step, Step::Inputs);
    }
}
