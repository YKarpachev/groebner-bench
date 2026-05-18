import json
import sys
from pathlib import Path
from typing import Dict, List


import pandas as pd

RowValue = str | int | float


def extract_data(folder_paths: List[Path]) -> List[List[Dict[str, RowValue]]]:
    whole_data: List[List[Dict[str, RowValue]]] = []

    for folder_path in folder_paths:
        folder = Path(folder_path)
        data: List[Dict[str, RowValue]] = []

        for json_path in folder.glob("*.json"):
            if not json_path.is_file():
                continue

            obj = json.loads(json_path.read_text())

            inner_dict: Dict[str, RowValue] = {
                "name": obj["test_name"],
                "status": obj["status"],
            }

            if inner_dict["status"] == "ok":
                metrics = obj["metrics"]

                inner_dict["solve_time_sec"] = obj["solver"]["solve_time_sec"]
                inner_dict["criterion_1_count"] = obj["solver"]["criterion_1_count"]
                inner_dict["criterion_2_count"] = obj["solver"]["criterion_2_count"]
                inner_dict["variable_count"] = obj["variable_count"]
                inner_dict["equation_count"] = obj["equation_count"]

                inner_dict["avg_coefficient"] = metrics["Average coefficient"]
                inner_dict["avg_deg_per_literal"] = metrics[
                    "Average degree per literal"
                ]
                inner_dict["avg_deg_per_monomial"] = metrics[
                    "Average degree per monomial"
                ]
                inner_dict["avg_literals_per_monomial"] = metrics[
                    "Average literals per monomial"
                ]
                inner_dict["avg_monomials_per_polynomial"] = metrics[
                    "Average monomials per polynomial"
                ]
                inner_dict["max_coefficient"] = metrics["Maximum coefficient"]
                inner_dict["max_degree"] = metrics["Maximum degree"]
                inner_dict["max_literals_per_monomial"] = metrics[
                    "Maximum literals per monomial"
                ]
                inner_dict["num_literals"] = metrics["Number of literals"]
                inner_dict["num_monomials"] = metrics["Number of monomials"]
                inner_dict["num_polynomials"] = metrics["Number of polynomials"]
                inner_dict["sum_lcm_degrees"] = metrics["Sum of LCM degrees"]
                inner_dict["sum_lcm_remainders_all_monomials"] = metrics[
                    "Sum of LCM remainders (all monomials)"
                ]
                inner_dict["sum_lcm_remainders_leading_monomials"] = metrics[
                    "Sum of LCM remainders (leading monomials)"
                ]
                inner_dict["sum_coefficients"] = metrics["Sum of coefficients"]
                inner_dict["sum_degrees"] = metrics["Sum of degrees"]
            else:
                for key in [
                    "solve_time_sec",
                    "criterion_1_count",
                    "criterion_2_count",
                    "variable_count",
                    "equation_count",
                    "avg_coefficient",
                    "avg_deg_per_literal",
                    "avg_deg_per_monomial",
                    "avg_literals_per_monomial",
                    "avg_monomials_per_polynomial",
                    "max_coefficient",
                    "max_degree",
                    "max_literals_per_monomial",
                    "num_literals",
                    "num_monomials",
                    "num_polynomials",
                    "sum_lcm_degrees",
                    "sum_lcm_remainders_all_monomials",
                    "sum_lcm_remainders_leading_monomials",
                    "sum_coefficients",
                    "sum_degrees",
                ]:
                    inner_dict[key] = "-"

            data.append(inner_dict)

        whole_data.append(data)

    return whole_data


def make_book(data: List[List[Dict[str, RowValue]]], folder_paths: List[Path]):
    with pd.ExcelWriter("book.xlsx", engine="xlsxwriter") as writer:
        for rows, path in zip(data, folder_paths):
            df = pd.DataFrame(rows)
            df.to_excel(writer, sheet_name=str(path.name[:31]), index=False)


def main():
    paths: List[Path] = list(map(Path, sys.argv[1:]))

    data = extract_data(paths)

    make_book(data, paths)


if __name__ == "__main__":
    main()
