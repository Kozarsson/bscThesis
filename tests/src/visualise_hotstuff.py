import matplotlib.pyplot as plt
import csv
import numpy as np
import os

def extract_latency_from_csv(csv_path):
    """
    Reads the CSV and returns the latency values in milliseconds.
    Works for both Multisig (view,latency_s) and ROAST (view,latency_s,session_count).
    """
    latencies = []
    try:
        with open(csv_path, newline='') as csvfile:
            reader = csv.DictReader(csvfile)
            for row in reader:
                val_latency = row.get("latency_s")
                if val_latency:
                    latencies.append(float(val_latency) * 1000)  # sec → ms
    except FileNotFoundError:
        print(f"Warning: CSV file not found at {csv_path}. Skipping.")
    return np.array(latencies)

def get_median_latency(csv_path):
    """Return median latency in ms, or NaN if no data."""
    data = extract_latency_from_csv(csv_path)
    if data.size > 0:
        return np.median(data)
    return np.nan

# Only include sizes that actually have CSVs
SYSTEM_SIZES = [
    (3, 4),
    (7, 10),
    (21, 30),
    (34, 50),
    (67, 100),
]

x_labels = [f"{t}/{n}" for t, n in SYSTEM_SIZES]
roast_data = []
multisig_data = []

# Collect data
base_root = "/Users/matyaskozar/Code/Thesis/benchmark-results"

for threshold, system_size in SYSTEM_SIZES:
    base_path = f"{base_root}/{threshold}-out-of-{system_size}/hotstuff"

    roast_path = os.path.join(base_path, "roast", "qc_latency.csv")
    multisig_path = os.path.join(base_path, "multisig", "qc_latency.csv")

    roast_data.append(get_median_latency(roast_path))
    multisig_data.append(get_median_latency(multisig_path))

# Plot
plt.figure(figsize=(12, 6))
x = np.arange(len(x_labels))
bar_width = 0.35

bars_roast = plt.bar(x - bar_width/2, roast_data, width=bar_width, label="ROAST", color="steelblue")
bars_multisig = plt.bar(x + bar_width/2, multisig_data, width=bar_width, label="Multisig", color="indianred")

# Annotate median values above each bar
def annotate_bars(bars):
    for bar in bars:
        height = bar.get_height()
        if not np.isnan(height):  # skip NaN bars
            plt.text(
                bar.get_x() + bar.get_width()/2,
                height,
                f"{height:.3f}",
                ha="center",
                va="bottom",
                fontsize=10,
                fontweight="bold"
            )

annotate_bars(bars_roast)
annotate_bars(bars_multisig)

plt.xlabel("Threshold/System Size", fontsize=14)
plt.ylabel("QC Latency (ms)", fontsize=14)
plt.xticks(x, x_labels, rotation=45, ha='right', fontsize=12)
plt.yticks(fontsize=12)
plt.grid(axis='y', linestyle='--', alpha=0.7)
plt.legend(fontsize=14, loc="upper left")

plt.tight_layout()
plt.show()
