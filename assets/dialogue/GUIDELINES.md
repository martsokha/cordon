# Dialogue authoring guidelines

How yarn dialogue interacts with the bunker dialogue UI, and the conventions to follow when authoring new NPC conversations.

## How the UI renders dialogue

The UI shows a speaker name, a block of text, and a row of buttons. Each yarn event maps to a UI state:

- `PresentLine` → text is shown with a single **Continue** button. Player clicks Continue to advance.
- `PresentOptions` → text is kept from the preceding line (as a prompt header) and the options become buttons.

The key behavior: **text persists across the Line → Options transition in the same conversation.** That lets a line serve as the prompt above the choices without re-authoring it.

Freshly entering options from `Idle` (a new conversation, or a step-away resume) clears text instead — the previous line was said in a previous session.

## Metadata tags

### `#autocontinue`

On a line that immediately precedes an options block in the same node.

The runner advances past the line instantly (no Continue button), and the text is captured as the prompt header for the upcoming options. Use this for state-dependent prompts like:

```yarn
<<if not $done_medkit and not $done_ration>>
    Sergeant: I need a medkit and a ration. #autocontinue
<<endif>>
-> Here's a medkit. ...
-> Here's a ration. ...
```

Without the tag, the player has to click Continue before seeing the options.

### `#transient`

On a response line that shouldn't linger as context above the next options block.

Transient lines are still shown and still require a Continue click, but their text is cleared when the following options appear — so "Appreciate it. Rations keep my boys fed." doesn't hang as a stale header above the next menu.

```yarn
-> Here's a ration.
    <<give_item "item_ration">>
    Sergeant: Appreciate it. #transient
    <<jump sergeant_trade_menu>>
```

(In practice, if a response line immediately jumps back to a menu, it's often cleaner to just delete the response and let the menu's own state-dependent header take over.)

### `#hide`

On an option whose `<<if>>` guard failed.

Greyed-out options are normally rendered disabled; `#hide` makes them disappear entirely. Used to swap an option in/out based on inventory state without showing both side-by-side.

```yarn
-> Here's a medkit. <<if not $done_medkit and $carrying == "item_medkit">> #hide
```

## Writing state-dependent menus

Menus that the player visits repeatedly (e.g. a trade menu with a step-away option) should branch on state to vary the prompt line. Use `<<if>> / <<elseif>> / <<else>>` and tag each variant with `#autocontinue`:

```yarn
<<if $done_medkit and $done_ration>>
    Sergeant: That's everything. Good hunting.
    <<stop>>
<<elseif $done_medkit>>
    Sergeant: Kit's sorted. Still need that ration: 40 credits. #autocontinue
<<elseif $done_ration>>
    Sergeant: Rations covered. Still need the medkit: 200 credits. #autocontinue
<<else>>
    Sergeant: I need a medkit and a ration. #autocontinue
<<endif>>
-> Here's a medkit. ...
```

This gives the NPC something fresh to say every time the player returns, without requiring an extra Continue click.

## Step-away targets

A node referenced by `<<step_away "node_name">>` is re-entered from scratch when the visitor returns. The UI clears any prior text on that re-entry, so the step-away target **must** begin with a line (usually `#autocontinue`-tagged) — otherwise the player sees options with no prompt.

## Style

- Prefer `:` over `—` for clause separation in dialogue text.
- Keep lines short. Three short lines with Continue clicks beats one long paragraph.
- Don't author response lines ("Appreciate it.", "Good work.") unless they add flavor. A silent handoff that returns to a state-dependent menu prompt usually reads better than a response line the player has to click past.
