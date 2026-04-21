//! Trade tab: suppliers list, catalog, and daily expense report.

use bevy::prelude::*;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::entity::player::ExpenseKind;
use cordon_core::item::Item;
use cordon_core::primitive::{Credits, Id};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::LastDailyExpenses;
use cordon_sim::quest::messages::PlaceOrderRequest;
use cordon_sim::resources::{PendingOrders, PlayerIdentity, PlayerSuppliers};

use super::{LaptopTab, TabContent};
use crate::ui::UiFont;
use crate::locale::L10n;
use crate::{AppState, PlayingState};

#[derive(Component)]
struct ExpensePanel;

#[derive(Component)]
struct SuppliersPanel;

#[derive(Component)]
struct CatalogPanel;

/// Which supplier's catalog is currently displayed on the right
/// panel. `None` means no supplier selected yet (show the prompt).
#[derive(Resource, Default, Debug)]
struct SelectedSupplier(Option<Id<NpcTemplate>>);

/// Button in the suppliers list. Clicking sets
/// [`SelectedSupplier`] to this template.
#[derive(Component)]
struct SupplierButton(Id<NpcTemplate>);

/// Button in the catalog list. Clicking fires a
/// [`PlaceOrderRequest`] for this item from the selected supplier.
#[derive(Component)]
struct OrderButton {
    item: Id<Item>,
    supplier: Id<NpcTemplate>,
    price: Credits,
    enabled: bool,
}

const HEADING_COLOR: Color = Color::srgb(0.8, 0.8, 0.6);
const DIM_COLOR: Color = Color::srgba(0.5, 0.5, 0.5, 0.8);
const NORMAL_COLOR: Color = Color::srgb(0.75, 0.75, 0.7);
const RED_COLOR: Color = Color::srgb(0.9, 0.3, 0.3);
const BUTTON_BG: Color = Color::srgba(0.12, 0.12, 0.15, 0.9);
const BUTTON_BG_SELECTED: Color = Color::srgba(0.2, 0.2, 0.25, 1.0);
const BUTTON_BG_DISABLED: Color = Color::srgba(0.08, 0.08, 0.1, 0.9);

pub struct TradeUiPlugin;

impl Plugin for TradeUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedSupplier>();
        app.add_systems(
            Update,
            (
                refresh_expense_panel,
                refresh_suppliers_panel,
                refresh_catalog_panel,
                handle_supplier_clicks,
                handle_order_clicks,
            )
                .run_if(in_state(PlayingState::Laptop)),
        );
        // Clear the selection when a new run starts so the right
        // panel doesn't linger on the previous run's supplier.
        app.add_systems(OnEnter(AppState::Playing), reset_selection);
    }
}

fn reset_selection(mut selected: ResMut<SelectedSupplier>) {
    selected.0 = None;
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>, laptop_cam: Entity) {
    commands
        .spawn((
            TabContent(LaptopTab::Trade),
            bevy::ui::UiTargetCamera(laptop_cam),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                right: Val::Px(16.0),
                top: Val::Px(40.0),
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Row,
                padding: UiRect::new(Val::Px(16.0), Val::Px(16.0), Val::Px(16.0), Val::Px(16.0)),
                column_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.85)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Left panel: suppliers list + daily expenses.
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(30.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|col| {
                    col.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|top| {
                        top.spawn((
                            Text::new("SUPPLIERS"),
                            TextFont {
                                font: font.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(HEADING_COLOR),
                        ));
                        top.spawn((
                            SuppliersPanel,
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(3.0),
                                ..default()
                            },
                        ));
                    });

                    // Daily expenses panel (bottom of left column).
                    col.spawn((
                        ExpensePanel,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(3.0),
                            padding: UiRect::top(Val::Px(8.0)),
                            border: UiRect::top(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.4, 0.4, 0.3, 0.4)),
                    ));
                });

            // Right panel: catalog for the selected supplier.
            parent.spawn((
                CatalogPanel,
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(70.0),
                    row_gap: Val::Px(6.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
            ));
        });
}

/// Rebuild the suppliers list when either the unlock set or the
/// selection changes. Cheap (few rows), so we redraw instead of
/// diffing. Only unlocked suppliers render — locked ones stay
/// hidden until unlocked.
fn refresh_suppliers_panel(
    mut commands: Commands,
    l10n: L10n,
    suppliers: Res<PlayerSuppliers>,
    selected: Res<SelectedSupplier>,
    font: Res<UiFont>,
    panel_q: Query<(Entity, Option<&Children>), With<SuppliersPanel>>,
) {
    if !suppliers.is_changed() && !selected.is_changed() {
        return;
    }
    let Ok((panel, children)) = panel_q.single() else {
        return;
    };
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    if suppliers.is_empty() {
        let none = spawn_text(
            &mut commands,
            "No suppliers available.",
            11.0,
            &font.0,
            DIM_COLOR,
        );
        commands.entity(panel).add_child(none);
        return;
    }

    for template_id in suppliers.iter() {
        let name = l10n.get(&template_id.as_str().replace('_', "-"));
        let is_selected = selected.0.as_ref() == Some(template_id);
        let bg = if is_selected {
            BUTTON_BG_SELECTED
        } else {
            BUTTON_BG
        };
        let button = commands
            .spawn((
                SupplierButton(template_id.clone()),
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(bg),
            ))
            .id();
        let label = spawn_text(&mut commands, &name, 11.0, &font.0, NORMAL_COLOR);
        commands.entity(button).add_child(label);
        commands.entity(panel).add_child(button);
    }
}

/// Rebuild the catalog panel for the currently-selected supplier.
/// Re-runs when the selection changes, when player credits change
/// (so disabled/enabled states refresh), or when the unlock set
/// changes (selected supplier was just unlocked).
#[allow(clippy::too_many_arguments)]
fn refresh_catalog_panel(
    mut commands: Commands,
    data: Res<GameDataResource>,
    l10n: L10n,
    selected: Res<SelectedSupplier>,
    identity: Res<PlayerIdentity>,
    orders: Res<PendingOrders>,
    font: Res<UiFont>,
    panel_q: Query<(Entity, Option<&Children>), With<CatalogPanel>>,
) {
    if !selected.is_changed() && !identity.is_changed() && !orders.is_changed() {
        return;
    }
    let Ok((panel, children)) = panel_q.single() else {
        return;
    };
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let Some(supplier_id) = selected.0.clone() else {
        let prompt = spawn_text(
            &mut commands,
            "SELECT A SUPPLIER",
            13.0,
            &font.0,
            Color::srgba(0.5, 0.5, 0.5, 0.6),
        );
        commands.entity(panel).add_child(prompt);
        return;
    };

    let Some(template) = data.0.npc_templates.get(&supplier_id) else {
        return;
    };
    let Some(supplier_info) = template.supplier.as_ref() else {
        return;
    };

    // Header with supplier name + count of orders pending *from
    // this supplier specifically*. A global count would mislead
    // when multiple suppliers have in-flight orders.
    let pending_from_here = orders.iter().filter(|o| o.supplier == supplier_id).count();
    let header_text = if pending_from_here > 0 {
        format!(
            "{} — {} order(s) in transit",
            l10n.get(&supplier_id.as_str().replace('_', "-")),
            pending_from_here
        )
    } else {
        l10n.get(&supplier_id.as_str().replace('_', "-"))
    };
    let header = spawn_text(&mut commands, &header_text, 13.0, &font.0, HEADING_COLOR);
    commands.entity(panel).add_child(header);

    // Items listing this supplier.
    let mut items: Vec<_> = data
        .0
        .items
        .iter()
        .filter(|(_, def)| def.suppliers.contains(&supplier_id))
        .collect();
    items.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

    if items.is_empty() {
        let none = spawn_text(
            &mut commands,
            "No stock right now.",
            11.0,
            &font.0,
            DIM_COLOR,
        );
        commands.entity(panel).add_child(none);
        return;
    }

    for (item_id, item_def) in items {
        let price = supplier_info.price_for(item_def.base_price);
        let affordable = identity.credits.can_afford(price);
        let name = l10n.get(item_id.as_str());
        let row = spawn_order_row(
            &mut commands,
            &name,
            price,
            item_id.clone(),
            supplier_id.clone(),
            affordable,
            &font.0,
        );
        commands.entity(panel).add_child(row);
    }
}

fn spawn_order_row(
    commands: &mut Commands,
    name: &str,
    price: Credits,
    item: Id<Item>,
    supplier: Id<NpcTemplate>,
    affordable: bool,
    font: &Handle<Font>,
) -> Entity {
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            ..default()
        })
        .id();
    let color = if affordable { NORMAL_COLOR } else { DIM_COLOR };
    let name_label = spawn_text(commands, name, 11.0, font, color);
    let price_label = spawn_text(commands, &format!("{} ¤", price.value()), 11.0, font, color);
    let button_bg = if affordable {
        BUTTON_BG
    } else {
        BUTTON_BG_DISABLED
    };
    let button = commands
        .spawn((
            OrderButton {
                item,
                supplier,
                price,
                enabled: affordable,
            },
            Button,
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(button_bg),
        ))
        .id();
    let button_label = spawn_text(commands, "Order", 11.0, font, color);
    commands.entity(button).add_child(button_label);
    commands
        .entity(row)
        .add_children(&[name_label, price_label, button]);
    row
}

fn handle_supplier_clicks(
    mut selected: ResMut<SelectedSupplier>,
    interactions: Query<(&Interaction, &SupplierButton), Changed<Interaction>>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            selected.0 = Some(button.0.clone());
        }
    }
}

fn handle_order_clicks(
    interactions: Query<(&Interaction, &OrderButton), Changed<Interaction>>,
    mut place_tx: MessageWriter<PlaceOrderRequest>,
) {
    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed || !button.enabled {
            continue;
        }
        place_tx.write(PlaceOrderRequest {
            item: button.item.clone(),
            supplier: button.supplier.clone(),
            expected_price: button.price,
        });
    }
}

fn refresh_expense_panel(
    mut commands: Commands,
    expenses: Res<LastDailyExpenses>,
    identity: Res<PlayerIdentity>,
    font: Res<UiFont>,
    panel_q: Query<(Entity, Option<&Children>), With<ExpensePanel>>,
) {
    if !expenses.is_changed() && !identity.is_changed() {
        return;
    }
    let Ok((panel, children)) = panel_q.single() else {
        return;
    };

    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let font_handle = font.0.clone();

    let heading = spawn_text(
        &mut commands,
        "DAILY EXPENSES",
        11.0,
        &font_handle,
        HEADING_COLOR,
    );
    commands.entity(panel).add_child(heading);

    let Some(report) = &expenses.0 else {
        let none = spawn_text(&mut commands, "No data yet.", 10.0, &font_handle, DIM_COLOR);
        commands.entity(panel).add_child(none);
        return;
    };

    for line in &report.lines {
        let label = match line.kind {
            ExpenseKind::SquadUpkeep => "Squad upkeep",
            ExpenseKind::GarrisonBribe => "Garrison bribe",
            ExpenseKind::SyndicateInterest => "Syndicate interest",
        };
        let row = spawn_expense_row(
            &mut commands,
            label,
            line.amount.value(),
            &font_handle,
            NORMAL_COLOR,
        );
        commands.entity(panel).add_child(row);
    }

    let sep = commands
        .spawn(Node {
            height: Val::Px(1.0),
            width: Val::Percent(100.0),
            margin: UiRect::vertical(Val::Px(2.0)),
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.4, 0.4, 0.3, 0.3)))
        .id();
    commands.entity(panel).add_child(sep);

    let total_row = spawn_expense_row(
        &mut commands,
        "Total",
        report.total.value(),
        &font_handle,
        NORMAL_COLOR,
    );
    commands.entity(panel).add_child(total_row);

    let debt = identity.debt.value();
    if debt > 0 {
        let debt_row = spawn_expense_row(
            &mut commands,
            "Outstanding debt",
            debt,
            &font_handle,
            RED_COLOR,
        );
        commands.entity(panel).add_child(debt_row);
    }

    let bal_color = if identity.credits.value() == 0 && debt > 0 {
        RED_COLOR
    } else {
        NORMAL_COLOR
    };
    let bal_row = spawn_expense_row(
        &mut commands,
        "Balance",
        identity.credits.value(),
        &font_handle,
        bal_color,
    );
    commands.entity(panel).add_child(bal_row);
}

fn spawn_text(
    commands: &mut Commands,
    text: &str,
    size: f32,
    font: &Handle<Font>,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            TextFont {
                font: font.clone(),
                font_size: size,
                ..default()
            },
            TextColor(color),
        ))
        .id()
}

fn spawn_expense_row(
    commands: &mut Commands,
    label: &str,
    amount: u32,
    font: &Handle<Font>,
    color: Color,
) -> Entity {
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            width: Val::Percent(100.0),
            ..default()
        })
        .id();
    let left = spawn_text(commands, label, 10.0, font, color);
    let right = spawn_text(commands, &format!("{amount} ¤"), 10.0, font, color);
    commands.entity(row).add_children(&[left, right]);
    row
}
