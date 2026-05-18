from pathlib import Path
from typing import Dict
import json
import sys

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt


def extract_data(folder_path: Path) -> Dict[str, float | None]:
    folder = Path(folder_path)

    data: Dict[str, float | None] = {}

    for path in folder.glob("*.json"):
        if not path.is_file():
            continue

        obj = json.loads(path.read_text())

        name: str = obj["test_name"]
        status: str = obj["status"]
        time: float | None = None
        if status == "ok":
            time = obj["solver"]["solve_time_sec"]

        data[name] = time

    return data


def make_table(
    previous_data: Dict[str, float | None],
    new_data: Dict[str, float | None],
) -> pd.DataFrame:
    rows = []

    all_tests = sorted(set(previous_data) & set(new_data))

    for test_name in all_tests:
        previous_time = previous_data.get(test_name, None)
        new_time = new_data.get(test_name, None)

        diff: float | None = None
        ratio: float | None = None

        if previous_time is not None and new_time is not None:
            diff = new_time - previous_time
            ratio = new_time / previous_time if previous_time != 0 else None

        rows.append(
            {
                "test_name": test_name,
                "previous_time": previous_time,
                "new_time": new_time,
                "diff": diff,
                "ratio": ratio,
            }
        )

    return pd.DataFrame(rows)


def make_graph(dataframe: pd.DataFrame, target_dir: Path) -> None:
    items = []

    for _, row in dataframe.iterrows():
        test_name = row["test_name"]
        previous_time = row["previous_time"]
        new_time = row["new_time"]

        previous_ok = pd.notna(previous_time)
        new_ok = pd.notna(new_time)

        if previous_ok and new_ok:  # pyright: ignore[reportGeneralTypeIssues]
            if previous_time <= 0 or new_time <= 0:
                continue

            value = np.log2(previous_time / new_time)

            if np.isclose(value, 0.0):
                kind = "same"
                color = None
            elif value < 0:
                kind = "slowdown"
                color = "#F59E0B"
            else:
                kind = "speedup"
                color = "#2563EB"

            items.append(
                {
                    "test_name": test_name,
                    "value": value,
                    "kind": kind,
                    "color": color,
                }
            )

        elif previous_ok and not new_ok:  # pyright: ignore[reportGeneralTypeIssues]
            items.append(
                {
                    "test_name": test_name,
                    "value": 0.0,
                    "kind": "red_cross",
                    "color": "#DC2626",
                }
            )

        elif not previous_ok and new_ok:  # pyright: ignore[reportGeneralTypeIssues]
            items.append(
                {
                    "test_name": test_name,
                    "value": 0.0,
                    "kind": "green_dot",
                    "color": "#059669",
                }
            )

        else:
            items.append(
                {
                    "test_name": test_name,
                    "value": 0.0,
                    "kind": "empty",
                    "color": None,
                }
            )

    if not items:
        print("Nothing to plot.")
        return

    def sort_key(item: dict) -> tuple:
        kind = item["kind"]
        value = item["value"]

        if kind == "slowdown":
            return (0, value)

        if kind in {"red_cross", "same", "green_dot", "empty"}:
            baseline_order = {
                "red_cross": 0,
                "same": 1,
                "green_dot": 2,
                "empty": 3,
            }
            return (1, baseline_order[kind], item["test_name"])

        if kind == "speedup":
            return (2, value)

        return (3, 0)

    items.sort(key=sort_key)

    x = np.arange(len(items))

    max_name_len = max(len(str(item["test_name"])) for item in items)
    fig_width = max(12, len(items) * 0.18, max_name_len * 0.12)
    fig_height = 7

    plt.figure(figsize=(fig_width, fig_height), layout="constrained")

    plt.axhline(0, color="#111827", linewidth=1)

    bar_x = []
    bar_y = []
    bar_colors = []
    cross_x = []
    dot_x = []

    for i, item in enumerate(items):
        if item["kind"] in {"slowdown", "speedup"}:
            bar_x.append(i)
            bar_y.append(item["value"])
            bar_colors.append(item["color"])
        elif item["kind"] == "red_cross":
            cross_x.append(i)
        elif item["kind"] == "green_dot":
            dot_x.append(i)

    if bar_x:
        plt.bar(bar_x, bar_y, color=bar_colors, width=0.7)

    if cross_x:
        plt.scatter(
            cross_x,
            np.zeros(len(cross_x)),
            color="#DC2626",
            marker="x",
            s=140,
            linewidths=3,
            zorder=3,
        )

    if dot_x:
        plt.scatter(
            dot_x,
            np.zeros(len(dot_x)),
            color="#059669",
            marker="o",
            s=90,
            zorder=3,
        )

    plt.xticks(
        x,
        [item["test_name"] for item in items],
        rotation=90,
        fontsize=9,
    )

    plt.grid(axis="y", alpha=0.25)
    plt.ylabel("log₂(previous_time / new_time)")

    target_dir.mkdir(parents=True, exist_ok=True)
    plt.savefig(target_dir / "comparison.png", dpi=150, bbox_inches="tight")
    plt.close()


def main():
    path1: Path = Path(sys.argv[1])
    path2: Path = Path(sys.argv[2])
    target_dir: Path = Path(sys.argv[3])

    previous_data = extract_data(path1)
    new_data = extract_data(path2)

    df = make_table(previous_data, new_data)

    make_graph(df, target_dir)


if __name__ == "__main__":
    main()
