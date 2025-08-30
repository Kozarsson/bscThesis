import matplotlib.pyplot as plt
import csv
import numpy as np

def extract_measurements_from_csv(csv_path):
    """
    Reads the CSV and returns the sample_measured_value per iteration as a numpy array.
    The raw 'sample_measured_value' is in nanoseconds.
    """
    values = []
    try:
        with open(csv_path, newline='') as csvfile:
            reader = csv.DictReader(csvfile)
            for row in reader:
                val_sample = row.get('sample_measured_value')
                val_iter = row.get('iteration_count')
                if val_sample and val_iter and float(val_iter) > 0:
                    values.append(float(val_sample) / float(val_iter))
    except FileNotFoundError:
        print(f"Warning: CSV file not found at {csv_path}. Skipping this data point.")
    return np.array(values)

def scale_time_data(data_ns):
    """
    Scales time data (assumed to be in nanoseconds) to milliseconds (ms).
    """
    return data_ns / 1000000, "ms"


def get_median_time(csv_path):
    """
    Extracts measurements, scales them to milliseconds, and returns the median.
    Returns None if no data is found.
    """
    data_ns = extract_measurements_from_csv(csv_path)
    if data_ns.size > 0:
        scaled_data, _ = scale_time_data(data_ns)
        return np.median(scaled_data)
    return None

# Define the system sizes
SYSTEM_SIZES = [
    (3, 4),    # threshold/system_size
    (7, 10),
    (21, 30),
    (34, 50),
    (67, 100),
    (334, 500),
    (667, 1000)
]

# Define the phases and their corresponding file paths and line styles
# EXCLUDING "initiation" phase and "Multisig Aggregation"
PHASES_CONFIG = {
    "signing": {
        "frost": {"label": "FROST Signing", "color": "skyblue"}, # Changed to color for bars
        "multisig": {"label": "Multisig Signing", "color": "lightcoral"} # Changed to color for bars
    },
    "aggregation": { # Only FROST aggregation exists
        "frost": {"label": "FROST Aggregation", "color": "cornflowerblue"} # Changed to color for bars
    },
    "verify": {
        "frost": {"label": "FROST Verification", "color": "blue"}, # Changed to color for bars
        "multisig": {"label": "Multisig Verification", "color": "red"} # Changed to color for bars
    }
}

# Data structures to store median computation times
frost_data = {phase: [] for phase in PHASES_CONFIG}
multisig_data = {phase: [] for phase in PHASES_CONFIG}
x_labels = []

# Collect data for each system size
for threshold, system_size in SYSTEM_SIZES:
    x_labels.append(f"{threshold}/{system_size}")
    # IMPORTANT: Update this base_path if your data is not in the specified location
    base_path = f"/Users/matyaskozar/Desktop/benchmark results/{threshold}-out-of-{system_size}"

    for phase_key, protocols in PHASES_CONFIG.items():
        # FROST data
        frost_path = f"{base_path}/frost/frost_{phase_key}/base/raw.csv"
        frost_median = get_median_time(frost_path)
        frost_data[phase_key].append(frost_median if frost_median is not None else np.nan)

        # Multisig data (only if it exists for this phase)
        if "multisig" in protocols:
            multisig_path = f"{base_path}/multisig/multisig_{phase_key}/base/raw.csv"
            multisig_median = get_median_time(multisig_path)
            multisig_data[phase_key].append(multisig_median if multisig_median is not None else np.nan)
        else: # If multisig does not exist for this phase, append NaN to maintain list length
            multisig_data[phase_key].append(np.nan)

# Set up the plot
plt.figure(figsize=(14, 8)) # Increased figure size for better bar visibility

# Number of bars per group (FROST Signing, Multisig Signing, FROST Aggregation, FROST Verification, Multisig Verification)
# We need to count the actual number of data series that will be plotted.
num_series = 0
series_keys = [] # To store the ordered keys for plotting
for phase_key, protocols in PHASES_CONFIG.items():
    if "frost" in protocols:
        num_series += 1
        series_keys.append((phase_key, "frost"))
    if "multisig" in protocols:
        num_series += 1
        series_keys.append((phase_key, "multisig"))

bar_width = 0.8 / num_series # Adjust width based on number of series
x = np.arange(len(x_labels)) # The label locations

# Plot bars
for i, (phase_key, protocol_type) in enumerate(series_keys):
    if protocol_type == "frost":
        data_to_plot = frost_data[phase_key]
        label = PHASES_CONFIG[phase_key]["frost"]["label"]
        color = PHASES_CONFIG[phase_key]["frost"]["color"]
    else: # protocol_type == "multisig"
        data_to_plot = multisig_data[phase_key]
        label = PHASES_CONFIG[phase_key]["multisig"]["label"]
        color = PHASES_CONFIG[phase_key]["multisig"]["color"]

    offset = bar_width * i - (bar_width * (num_series - 1) / 2) # Calculate offset for grouping bars
    plt.bar(x + offset, data_to_plot, width=bar_width, label=label, color=color, alpha=0.9)


# Customize the plot
plt.xlabel("Threshold/System Size", fontsize=14)
plt.ylabel("Computation Time (ms)", fontsize=14)
plt.xticks(x, x_labels, rotation=45, ha='right', fontsize=12) # Set x-ticks at the group center
plt.yticks(fontsize=12)
plt.grid(axis='y', linestyle='--', alpha=0.7) # Grid on y-axis only for bar plots

# --- CHANGES START HERE ---
# Move the legend inside the plot to the top-left corner and increase its size
plt.legend(loc='upper left', bbox_to_anchor=(0.01, 0.99), fontsize=10 * 2) # Doubled fontsize
# --- CHANGES END HERE ---

plt.tight_layout()
plt.show()