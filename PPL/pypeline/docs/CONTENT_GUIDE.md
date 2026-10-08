# Writing Manual chapters and contracts

The Engineering Manual (F2) is the game's campaign. Every chapter teaches one Python idea and ends with contracts that pay coins and unlock the next chapter. Chapters are data, so you can write one without touching Rust beyond a single line.

## Files

Each chapter is two files with the same name:

| File | What it holds |
|------|---------------|
| `assets/data/manual/chNN_topic.md` | The lesson: text and runnable examples. |
| `assets/data/contracts/chNN_topic.ron` | The chapter's contracts. |

Both are built into the game with `include_str!` in `src/progression/chapters.rs`. To add a chapter, add one line to the list there with its number and the two files.

## The lesson (Markdown)

- The first `# Heading` is the chapter's title, for example `# 3. for Loops`.
- `## Headings` split the lesson into sections. Plain paragraphs become text. `**bold**` and `` `code` `` work as usual.
- Fenced ```` ```python ```` blocks become examples the player can insert into their code with one click.
- An example whose first line is `# snippet: Name` also goes into the **Snippets** menu once the chapter is done.
- An example whose first line is `# file: lines.py` is a whole file of its own. Inserting it creates that file, and later examples can `import lines`.

Keep it friendly and short: a new idea, a tiny example, then an example that does something visible on the island. Every example is run by the tests, so it must work as written.

## Contracts (RON)

The file is a list of contracts:

```ron
[
    (
        id: "ch3_long_haul",
        title: "Long Haul",
        brief: "Run a belt at least 6 tiles long with a for loop, and make 10 iron plates. Keep the script under 12 lines.",
        goal: Produce(item: IronPlate, count: 10),
        requires: [ForLoop],
        max_lines: Some(12),
        reward: 30,
        hints: [
            "for x in range(1, 7): places belts on x = 1 to 6.",
            "Put conveyors.place(x=x, y=0, dir=\"east\") inside the loop, indented by 4 spaces.",
        ],
        solution: r#"
from auto import conveyors, machines
...
"#,
    ),
]
```

| Field | Required | Meaning |
|-------|----------|---------|
| `id` | yes | Unique, like `ch3_long_haul`. Saved in players' progress, so never change it once released. |
| `title` | yes | Shown in the Manual and on the success banner. |
| `brief` | yes | What the player must do, in one or two sentences. |
| `goal` | yes | `Produce(item: IronPlate, count: 10)`, `Earn(coins: 40)` or `Rate(item: IronPlate, per_minute: 40)`. Items: `IronOre`, `IronPlate`. |
| `requires` | no | Python ideas the script must use: `Variables`, `FStrings`, `ForLoop`, `Conditionals`, `WhileLoop`, `Functions`, `Lists`, `Dicts`, `Modules`, `Events`, `Tick`, `Generators`. Checked from the script's syntax tree. |
| `max_lines` | no | `Some(12)`: most lines of code allowed. Blank lines and comments are free. |
| `reward` | yes | Coins paid on completion (more than 0). |
| `hints` | yes | Opened one at a time, from a gentle nudge to nearly the answer. At least one. |
| `solution` | yes | A script that beats the contract. CI runs it, so every contract is proven beatable. |
| `files` | no | Other files the solution imports: `{"lines": "def build(): ..."}` for `lines.py`. |
| `chapter_test` | no | `true` makes passing this contract complete the whole chapter, so experienced coders can skip ahead. Each chapter has exactly one. |
| `keep_cool` | no | `true` restarts the contract whenever a steam generator overheats. |

## What the tests check

`cargo test` checks every chapter automatically:

- the Markdown and RON parse, chapters are numbered in order, and each has a title;
- every contract id is unique, every contract has hints and a reward, and each chapter has exactly one chapter test;
- every example in the lesson runs;
- every `solution` beats its own contract and uses the concepts it requires.

If a test fails, its message names the chapter and contract.

## Style

- Talk to the player as "you". Short sentences, plain words, no jargon before it is taught.
- Teach with the factory: every idea should change something the player can see on the island.
- One new idea per chapter. Contracts should need that idea, and `requires` makes sure they do.
