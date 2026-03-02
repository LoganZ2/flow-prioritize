import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import math

# Set seaborn style for academic aesthetic
sns.set_theme(style="whitegrid", context="paper", font_scale=1.2)

def calculate_erlang_c(c, rho):
    if rho >= 1.0:
        return 1.0
    c_rho = c * rho
    sum_terms = sum((c_rho ** k) / math.factorial(k) for k in range(c))
    last_term = (c_rho ** c) / (math.factorial(c) * (1 - rho))
    return last_term / (sum_terms + last_term)

def validate_theoretical_model(df_metrics, df_tasks, scenario_name, start_phase=3000, end_phase=6000):
    print(f"\n--- Theoretical vs Empirical Validation: {scenario_name} (Heavy Load Phase) ---")
    
    # Filter for the specific proposed scenario
    scenario_metrics = df_metrics[df_metrics['scenario'] == f"{scenario_name}_Proposed"]
    scenario_tasks = df_tasks[df_tasks['scenario'] == f"{scenario_name}_Proposed"]
    
    if scenario_metrics.empty or scenario_tasks.empty:
        print(f"No data found for {scenario_name}_Proposed")
        return

    phase_metrics = scenario_metrics[(scenario_metrics['timestamp_ms'] >= start_phase) & (scenario_metrics['timestamp_ms'] <= end_phase)]
    lambda_1 = phase_metrics['lambda_mouse'].mean() if not phase_metrics.empty else 0
    lambda_2 = phase_metrics['lambda_elephant'].mean() if not phase_metrics.empty else 0
    
    phase_tasks = scenario_tasks[(scenario_tasks['arrival_time_ms'] >= start_phase) & (scenario_tasks['arrival_time_ms'] <= end_phase) & (scenario_tasks['dropped'] == False)]
    emp_wq1 = phase_tasks[phase_tasks['flow_type'] == 'Mouse']['wait_time_ms'].mean()
    emp_wq2 = phase_tasks[phase_tasks['flow_type'] == 'Elephant']['wait_time_ms'].mean()
    
    if pd.isna(emp_wq1): emp_wq1 = 0.0
    if pd.isna(emp_wq2): emp_wq2 = 0.0

    c = 4
    E_S1 = 0.005 # 5ms
    E_S2 = 0.050 # 50ms
    
    rho_1 = (lambda_1 * E_S1) / c
    rho_2 = (lambda_2 * E_S2) / c
    rho_total = rho_1 + rho_2
    
    print(f"Observed Arrival Rates: Lambda_Mouse = {lambda_1:.2f} t/s, Lambda_Elephant = {lambda_2:.2f} t/s")
    print(f"System Load (rho): {rho_total:.2f}")
    
    if rho_total >= 1.0:
        print("System was overloaded (rho >= 1). Steady-state W_q is theoretically infinite.")
    else:
        E_S = (lambda_1 * E_S1 + lambda_2 * E_S2) / (lambda_1 + lambda_2) if (lambda_1 + lambda_2) > 0 else 0
        P_c = calculate_erlang_c(c, rho_total)
        
        Wq1_sec = (P_c * E_S) / (c * (1 - rho_1)) if (1 - rho_1) > 0 else float('inf')
        Wq2_sec = (P_c * E_S) / (c * (1 - rho_1) * (1 - rho_total)) if (1 - rho_1) > 0 and (1 - rho_total) > 0 else float('inf')
        
        theo_wq1 = Wq1_sec * 1000
        theo_wq2 = Wq2_sec * 1000
        
        print(f"Mouse Flow (High Prio)    -> Theoretical Wq: {theo_wq1:.2f} ms | Empirical Wq: {emp_wq1:.2f} ms")
        print(f"Elephant Flow (Low Prio)  -> Theoretical Wq: {theo_wq2:.2f} ms | Empirical Wq: {emp_wq2:.2f} ms")

def get_cdf(data):
    if len(data) == 0:
        return np.array([]), np.array([])
    x, counts = np.unique(data, return_counts=True)
    y = np.cumsum(counts) / len(data)
    # If the minimum wait time is > 0, we can start the curve at (0, 0) for continuity
    # But if there are already 0s, we DO NOT insert (0,0) to prevent the vertical line artifact.
    if x[0] > 0:
        x = np.insert(x, 0, 0.0)
        y = np.insert(y, 0, 0.0)
    return x, y

def plot_latency_cdf_for_scenario(df_tasks, scenario_name):
    print(f"Plotting Latency CDF for {scenario_name}...")
    
    # Filter datasets
    df_prop = df_tasks[(df_tasks['scenario'] == f"{scenario_name}_Proposed") & (df_tasks['dropped'] == False)]
    df_base = df_tasks[(df_tasks['scenario'] == f"{scenario_name}_FIFO") & (df_tasks['dropped'] == False)]
    df_sjf = df_tasks[(df_tasks['scenario'] == f"{scenario_name}_SJF") & (df_tasks['dropped'] == False)]
    
    if df_prop.empty or df_base.empty or df_sjf.empty:
        print(f"Missing data for scenario {scenario_name}, skipping CDF.")
        return

    # Extract wait times
    prop_mouse = df_prop[df_prop['flow_type'] == 'Mouse']['wait_time_ms'].values
    prop_eleph = df_prop[df_prop['flow_type'] == 'Elephant']['wait_time_ms'].values
    
    base_mouse = df_base[df_base['flow_type'] == 'Mouse']['wait_time_ms'].values
    sjf_eleph = df_sjf[df_sjf['flow_type'] == 'Elephant']['wait_time_ms'].values

    plt.figure(figsize=(9, 6))
    
    # Plot Base Mouse
    x_b, y_b = get_cdf(base_mouse)
    if len(x_b) > 0:
        plt.plot(x_b, y_b, label=f'Baseline Mouse (FIFO) (n={len(base_mouse)})', color='gray', linestyle='--', linewidth=2.5)
        
    # Plot Proposed Mouse
    x_pm, y_pm = get_cdf(prop_mouse)
    if len(x_pm) > 0:
        plt.plot(x_pm, y_pm, label=f'Proposed Mouse (M/M/4 w/ AC) (n={len(prop_mouse)})', color='blue', linewidth=3.0)

    # Plot Proposed Elephant
    x_pe, y_pe = get_cdf(prop_eleph)
    if len(x_pe) > 0:
        plt.plot(x_pe, y_pe, label=f'Proposed Elephant (n={len(prop_eleph)})', color='red', linewidth=2.5)

    # Plot SJF Elephant
    x_se, y_se = get_cdf(sjf_eleph)
    if len(x_se) > 0:
        plt.plot(x_se, y_se, label=f'SJF Elephant (Starvation) (n={len(sjf_eleph)})', color='purple', linestyle=':', linewidth=2.5)

    plt.title(f"Wait Time CDF: {scenario_name.replace('_', ' ')}", fontsize=14, fontweight='bold')
    plt.xlabel("Wait Time (ms) - Symlog Scale", fontsize=12)
    plt.ylabel("CDF Probability", fontsize=12)
    # Using linthresh=0.5 ensures that if there are very small >0 values they render well
    plt.xscale('symlog', linthresh=0.5)
    plt.grid(True, which="both", ls="--", linewidth=0.5)
    plt.legend(loc='lower right', fontsize=10)
    plt.tight_layout()
    plt.savefig(f"scripts/plot_latency_cdf_{scenario_name}.png", dpi=300)
    plt.close()

if __name__ == "__main__":
    df_metrics = pd.read_csv("experiment_data/all_system_metrics.csv")
    df_tasks = pd.read_csv("experiment_data/all_tasks_trace.csv")
    
    scenarios = ["Scenario_1_GameServer", "Scenario_2_Microservices"]
    
    for sc in scenarios:
        validate_theoretical_model(df_metrics, df_tasks, sc)
        plot_latency_cdf_for_scenario(df_tasks, sc)
        
    print("\nAll comparative analysis complete. Multiple scenario plots generated in scripts/ directory.")
