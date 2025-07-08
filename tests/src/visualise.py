import matplotlib.pyplot as plt
import csv
import numpy as np

def extract_measurements_from_csv(csv_path):
    """
    Reads the CSV and returns the sample_measured_value per iteration as a numpy array.
    The raw 'sample_measured_value' is in nanoseconds.
    """
    values = []
    with open(csv_path, newline='') as csvfile:
        reader = csv.DictReader(csvfile)
        for row in reader:
            val_sample = row.get('sample_measured_value')
            val_iter = row.get('iteration_count')
            if val_sample and val_iter and float(val_iter) > 0:
                # Calculate time per iteration in nanoseconds
                values.append(float(val_sample) / float(val_iter))
    return np.array(values)

def scale_time_data(data_ns):
    """
    Scales time data (assumed to be in nanoseconds) to microseconds (µs).
    """
    return data_ns / 1000, "µs" # Always convert to microseconds and return "µs"

def format_median_value(value):
    """
    Formats a numerical value to a maximum of 4 significant figures.
    If the number is 5 digits or larger (e.g., 10000.0 or more), uses scientific notation.
    """
    if abs(value) >= 10000 or (abs(value) > 0 and abs(value) < 0.001): # Check for large numbers or very small numbers
        return f'{value:.3e}' # 1 digit before decimal, 3 after for mantissa (4 sig figs total)
    else:
        return f'{value:.4g}' # General formatting to 4 significant figures

# System size and threshold used for the title
SYSTEM_SIZE = 1000
THRESHOLD = 667


csv_file_path_1 = f"/Users/matyaskozar/Desktop/benchmark results/{THRESHOLD}-out-of-{SYSTEM_SIZE}/frost/frost_initiation/base/raw.csv"
csv_file_path_2 = f"/Users/matyaskozar/Desktop/benchmark results/{THRESHOLD}-out-of-{SYSTEM_SIZE}/frost/frost_signing/base/raw.csv"
csv_file_path_3 = f"/Users/matyaskozar/Desktop/benchmark results/{THRESHOLD}-out-of-{SYSTEM_SIZE}/frost/frost_aggregation/base/raw.csv"
csv_file_path_4 = f"/Users/matyaskozar/Desktop/benchmark results/{THRESHOLD}-out-of-{SYSTEM_SIZE}/frost/frost_verify/base/raw.csv"
csv_file_path_5 = f"/Users/matyaskozar/Desktop/benchmark results/{THRESHOLD}-out-of-{SYSTEM_SIZE}/multisig/multisig_initiation/base/raw.csv"
csv_file_path_6 = f"/Users/matyaskozar/Desktop/benchmark results/{THRESHOLD}-out-of-{SYSTEM_SIZE}/multisig/multisig_signing/base/raw.csv"
csv_file_path_7 = f"/Users/matyaskozar/Desktop/benchmark results/{THRESHOLD}-out-of-{SYSTEM_SIZE}/multisig/multisig_verify/base/raw.csv"

# Extract data from each of the 7 CSV files (data must be in nanoseconds per iteration)
d1 = extract_measurements_from_csv(csv_file_path_1) # FROST Initiation
d2 = extract_measurements_from_csv(csv_file_path_2) # FROST Signing
d3 = extract_measurements_from_csv(csv_file_path_3) # FROST Aggregation
d4 = extract_measurements_from_csv(csv_file_path_4) # FROST Verification
d5 = extract_measurements_from_csv(csv_file_path_5) # Multisig Initiation
d6 = extract_measurements_from_csv(csv_file_path_6) # Multisig Signing
d7 = extract_measurements_from_csv(csv_file_path_7) # Multisig Verification


boxplot_widths = 0.5
whisker_color = 'darkgray'
whisker_linewidth = 2
whisker_linestyle = '-'
cap_color = 'darkgray'
cap_linewidth = 2
median_linewidth = 2.5

FROST_MEDIAN_LINE_COLOR = 'blue'
MULTISIG_MEDIAN_LINE_COLOR = 'red'
BOX_PATCH_COLOR = 'lightgray'

all_data_ns = [d1, d5, d2, d6, d4, d7, d3]
all_labels = [
    "FROST Initiation", "Multisig Initiation",
    "FROST Signing", "Multisig Signing",
    "FROST Verification", "Multisig Verification",
    "FROST Aggregation"
]

protocol_types = [
    "FROST", "Multisig",
    "FROST", "Multisig",
    "FROST", "Multisig",
    "FROST"
]


fig, ax = plt.subplots(1, 1, figsize=(15, 8)) 


fig.suptitle(f'Benchmark Performance for (t={THRESHOLD} out of n={SYSTEM_SIZE}) System', fontsize=18, y=0.98)

# Scale all data to microseconds
overall_scaled_data_list = []
overall_unit = "µs" 

for data_array_ns in all_data_ns:
    if data_array_ns.size > 0:
        # Always convert to microseconds
        overall_scaled_data_list.append(data_array_ns / 1000)
    else:
        overall_scaled_data_list.append(np.array([])) # Append empty array for empty data


plot_data = [d for d in overall_scaled_data_list if d.size > 0]
plot_labels = [all_labels[i] for i, d in enumerate(overall_scaled_data_list) if d.size > 0]
plot_protocol_types = [protocol_types[i] for i, d in enumerate(overall_scaled_data_list) if d.size > 0]

bp = ax.boxplot(
    plot_data,
    positions=np.arange(1, len(plot_data) + 1), 
    widths=boxplot_widths,
    patch_artist=True,
    showfliers=True 
)


for patch in bp['boxes']:
    patch.set_facecolor(BOX_PATCH_COLOR)

for whisker in bp['whiskers']:
    whisker.set(color=whisker_color, linewidth=whisker_linewidth, linestyle=whisker_linestyle)
for cap in bp['caps']:
    cap.set(color=cap_color, linewidth=cap_linewidth)

for i, median_line in enumerate(bp['medians']): 
    if plot_protocol_types[i] == "FROST":
        median_line.set(color=FROST_MEDIAN_LINE_COLOR, linewidth=median_linewidth)
    elif plot_protocol_types[i] == "Multisig":
        median_line.set(color=MULTISIG_MEDIAN_LINE_COLOR, linewidth=median_linewidth)


ax.set_ylim(bottom=0)

if len(plot_data) > 0 and np.concatenate(plot_data).size > 0:
    max_val_overall_scaled = np.max(np.concatenate(plot_data)) 
    y_max_padded = max_val_overall_scaled * 1.10 
    ax.set_ylim(top=y_max_padded)


for i, data_array_original_scale in enumerate(plot_data):
    if data_array_original_scale.size > 0:
        median_val = np.median(data_array_original_scale) 

        text_x_pos = np.arange(1, len(plot_data) + 1)[i]
        
        y_min_vis, y_max_vis = ax.get_ylim()
        y_range_vis = y_max_vis - y_min_vis
        
        median_text_offset_y = y_range_vis * 0.015 

        formatted_median = format_median_value(median_val)
        
        median_text_color = FROST_MEDIAN_LINE_COLOR if plot_protocol_types[i] == "FROST" else MULTISIG_MEDIAN_LINE_COLOR

        ax.text(text_x_pos, median_val + median_text_offset_y, formatted_median, ha='center', va='bottom', fontsize=8, weight='bold', color=median_text_color)


ax.set_xticks(np.arange(1, len(plot_data) + 1))
ax.set_xticklabels(plot_labels, rotation=45, ha='right') 
ax.set_ylabel(f"Time ({overall_unit})")
ax.set_title("Combined Benchmark Phases") 
ax.grid(axis='y', linestyle='--', alpha=0.7)

plt.tight_layout(rect=[0, 0.03, 1, 0.95]) 
plt.show()
