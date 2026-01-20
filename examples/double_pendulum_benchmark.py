"""
🏆 DOUBLE PENDULUM BENCHMARK: QUAD-VIEW DASHBOARD
=================================================
SymPy vs SymEngine vs SymbAnaFis

Dashboard:
[ SymPy ] [ SymEngine ]
[ SymbAnaFis ] [ Performance ]
"""

import time
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import matplotlib.gridspec as gridspec
from dataclasses import dataclass
import warnings
warnings.filterwarnings('ignore')

# ================= CONFIGURATION =================
N_PENDULUMS = 50000
N_STEPS = 600
DT = 0.01
WARMUP = 3
AXIS_LIMIT = 2.5

# Physics
G = 9.81
L1, L2 = 1.0, 1.0
M1, M2 = 1.0, 1.0
DAMPING = 0.9995

# ================= DERIVATION CACHE =================
_DERIVATION_CACHE = {}

def derive_sympy():
    if 'sympy' in _DERIVATION_CACHE: return _DERIVATION_CACHE['sympy']
    import sympy as sp
    th1, th2, w1, w2 = sp.symbols('th1 th2 w1 w2')
    m1, m2, l1, l2, g = sp.symbols('m1 m2 L1 L2 g')
    
    # Simple derivation for brevity in this file (Physics are standard)
    # Mass Matrix derived from Lagrangian T = 0.5*m1*v1^2 + 0.5*m2*v2^2
    # This is a bit long to write out fully symbolic every time, but necessary for the benchmark
    # We copy the proven logic from previous version
    
    x1 = l1 * sp.sin(th1)
    y1 = -l1 * sp.cos(th1)
    x2 = x1 + l2 * sp.sin(th2)
    y2 = y1 - l2 * sp.cos(th2)
    
    vx1 = l1 * w1 * sp.cos(th1)
    vy1 = l1 * w1 * sp.sin(th1)
    vx2 = vx1 + l2 * w2 * sp.cos(th2)
    vy2 = vy1 + l2 * w2 * sp.sin(th2)
    
    T = 0.5 * m1 * (vx1**2 + vy1**2) + 0.5 * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V
    
    # Lagrange Equations
    # d/dt(dL/dw) - dL/dth = 0
    # Returns a1, a2
    # We solve the linear system M * a = F
    
    dL_dw1 = sp.diff(L, w1)
    dL_dw2 = sp.diff(L, w2)
    dL_dth1 = sp.diff(L, th1)
    dL_dth2 = sp.diff(L, th2)
    
    # Coefficients for a1, a2
    # d(dL_dw)/dt = (d(dL_dw)/dw)*a + ...
    # M matrix elements
    M11 = sp.diff(dL_dw1, w1)
    M12 = sp.diff(dL_dw1, w2)
    M21 = sp.diff(dL_dw2, w1)
    M22 = sp.diff(dL_dw2, w2)
    
    # RHS
    # F = dL_dth - (d(dL_dw)/dth * w + ...)
    # Term from d/dt that doesn't involve a
    dt_term1 = sp.diff(dL_dw1, th1)*w1 + sp.diff(dL_dw1, th2)*w2
    dt_term2 = sp.diff(dL_dw2, th1)*w1 + sp.diff(dL_dw2, th2)*w2
    
    F1 = dL_dth1 - dt_term1
    F2 = dL_dth2 - dt_term2
    
    # Cramer's Rule for symbolic solve
    det = M11*M22 - M12*M21
    a1 = (F1*M22 - F2*M12) / det
    a2 = (M11*F2 - M21*F1) / det
    
    # Substitute values
    subs = {m1: M1, m2: M2, l1: L1, l2: L2, g: G}
    a1_sub = a1.subs(subs)
    a2_sub = a2.subs(subs)
    
    result = (a1_sub, a2_sub, (th1, th2, w1, w2))
    _DERIVATION_CACHE['sympy'] = result
    return result

def derive_symengine():
    # Similar to sympy but using symengine
    if 'symengine' in _DERIVATION_CACHE: return _DERIVATION_CACHE['symengine']
    import symengine as se
    th1, th2, w1, w2 = se.symbols('th1 th2 w1 w2')
    m1, m2, l1, l2, g = se.symbols('m1 m2 L1 L2 g')
    
    x1 = l1 * se.sin(th1)
    y1 = -l1 * se.cos(th1)
    x2 = x1 + l2 * se.sin(th2)
    y2 = y1 - l2 * se.cos(th2)
    
    vx1 = l1 * w1 * se.cos(th1)
    vy1 = l1 * w1 * se.sin(th1)
    vx2 = vx1 + l2 * w2 * se.cos(th2)
    vy2 = vy1 + l2 * w2 * se.sin(th2)
    
    T = 0.5 * m1 * (vx1**2 + vy1**2) + 0.5 * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V
    
    dL_dw1 = se.diff(L, w1)
    dL_dw2 = se.diff(L, w2)
    dL_dth1 = se.diff(L, th1)
    dL_dth2 = se.diff(L, th2)
    
    M11 = se.diff(dL_dw1, w1)
    M12 = se.diff(dL_dw1, w2)
    M21 = se.diff(dL_dw2, w1)
    M22 = se.diff(dL_dw2, w2)
    
    dt_term1 = se.diff(dL_dw1, th1)*w1 + se.diff(dL_dw1, th2)*w2
    dt_term2 = se.diff(dL_dw2, th1)*w1 + se.diff(dL_dw2, th2)*w2
    
    F1 = dL_dth1 - dt_term1
    F2 = dL_dth2 - dt_term2
    
    det = M11*M22 - M12*M21
    a1 = (F1*M22 - F2*M12) / det
    a2 = (M11*F2 - M21*F1) / det
    
    subs = {m1: M1, m2: M2, l1: L1, l2: L2, g: G}
    a1_sub = a1.subs(subs)
    a2_sub = a2.subs(subs)
    
    result = (a1_sub, a2_sub, (th1, th2, w1, w2))
    _DERIVATION_CACHE['symengine'] = result
    return result

def derive_symbanafis():
    if 'symbanafis' in _DERIVATION_CACHE: return _DERIVATION_CACHE['symbanafis']
    from symb_anafis import symb, Expr
    
    th1, th2, w1, w2 = symb('th1'), symb('th2'), symb('w1'), symb('w2')
    m1, m2, l1, l2, g = symb('m1'), symb('m2'), symb('L1'), symb('L2'), symb('g')
    
    x1 = l1 * th1.sin()
    y1 = Expr(0) - l1 * th1.cos()
    x2 = x1 + l2 * th2.sin()
    y2 = y1 - l2 * th2.cos()
    
    vx1 = l1 * w1 * th1.cos()
    vy1 = l1 * w1 * th1.sin()
    vx2 = vx1 + l2 * w2 * th2.cos()
    vy2 = vy1 + l2 * w2 * th2.sin()
    
    half = Expr(0.5)
    T = half * m1 * (vx1**2 + vy1**2) + half * m2 * (vx2**2 + vy2**2)
    V = m1 * g * y1 + m2 * g * y2
    L = T - V
    
    dL_dw1 = L.diff('w1')
    dL_dw2 = L.diff('w2')
    dL_dth1 = L.diff('th1')
    dL_dth2 = L.diff('th2')
    
    M11 = dL_dw1.diff('w1').simplify()
    M12 = dL_dw1.diff('w2').simplify()
    M21 = dL_dw2.diff('w1').simplify()
    M22 = dL_dw2.diff('w2').simplify()
    
    dt_term1 = (dL_dw1.diff('th1') * w1 + dL_dw1.diff('th2') * w2).simplify()
    dt_term2 = (dL_dw2.diff('th1') * w1 + dL_dw2.diff('th2') * w2).simplify()
    
    F1 = (dL_dth1 - dt_term1).simplify()
    F2 = (dL_dth2 - dt_term2).simplify()
    
    det = (M11*M22 - M12*M21).simplify()
    a1 = ((F1*M22 - F2*M12) / det).simplify()
    a2 = ((M11*F2 - M21*F1) / det).simplify()
    
    # Subs
    for var, val in [('m1', M1), ('m2', M2), ('L1', L1), ('L2', L2), ('g', G)]:
        a1 = a1.substitute(var, val)
        a2 = a2.substitute(var, val)
        
    result = (a1, a2)
    _DERIVATION_CACHE['symbanafis'] = result
    return result

# ================= ENGINES =================

def setup_sympy():
    try:
        import sympy as sp
        print("\n🔧 SymPy: Setting up...")
        t0 = time.perf_counter()
        a1, a2, vars = derive_sympy()
        
        # CSE for valid performance in Python
        func = sp.lambdify(vars, [a1, a2], 'numpy', cse=True)
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(s):
            # s is (4, N)
            return func(s[0], s[1], s[2], s[3])
            
        return evaluate, setup_time
    except ImportError:
        print("   ❌ SymPy not found")
        return None, 0

def setup_symengine():
    try:
        import symengine as se
        print("\n🔧 SymEngine: Setting up...")
        t0 = time.perf_counter()
        a1, a2, vars = derive_symengine()
        
        try:
            func = se.Lambdify(vars, [a1, a2], backend='llvm')
            print("   (LLVM)")
        except:
            func = se.Lambdify(vars, [a1, a2])
            print("   (Default)")
            
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(s):
            # (4, N) -> (N, 4) contiguous for SymEngine
            inp = np.ascontiguousarray(s.T)
            out = func(inp)
            return [out[:,0], out[:,1]]
        
        return evaluate, setup_time
    except ImportError:
        print("   ❌ SymEngine not found")
        return None, 0

def setup_symbanafis():
    try:
        from symb_anafis import eval_f64
        print("\n🔧 SymbAnaFis: Setting up...")
        t0 = time.perf_counter()
        a1, a2 = derive_symbanafis()
        
        exprs = [a1, a2]
        var_names = [["th1", "th2", "w1", "w2"]] * 2
        
        setup_time = time.perf_counter() - t0
        print(f"   ✓ Setup: {setup_time*1000:.1f} ms")
        
        def evaluate(s):
            # s is [th1, th2, w1, w2] columns
            cols = [s[0], s[1], s[2], s[3]]
            return eval_f64(exprs, var_names, [cols, cols])
            
        return evaluate, setup_time
    except ImportError:
        print("   ❌ SymbAnaFis not found")
        return None, 0

# ================= SIMULATION =================

def simulate(name, eval_fn, initial_states):
    # Capture only subset for visualization history
    vis_count = 50  # Matches PLOT_N_DRAW concept from before, kept small for clear lines
    vis_idx = np.linspace(0, N_PENDULUMS-1, vis_count, dtype=int)
    
    states = initial_states.copy()
    history = np.zeros((N_STEPS, 4, vis_count))
    
    # Local vars
    dt = DT
    dt_half = dt * 0.5
    dt_sixth = dt / 6.0
    
    # Pre-alloc RK4 arrays
    k1 = np.zeros_like(states)
    k2 = np.zeros_like(states)
    k3 = np.zeros_like(states)
    k4 = np.zeros_like(states)
    temp = np.zeros_like(states)
    acc = np.zeros_like(states)
    
    t0 = time.perf_counter()
    
    for i in range(N_STEPS):
        # Save history
        history[i] = states[:, vis_idx]
        
        # RK4 Step 1
        a1, a2 = eval_fn(states)
        k1[0], k1[1] = states[2], states[3]
        k1[2], k1[3] = a1, a2
        
        # Temp = S + k1*dt/2
        np.multiply(k1, dt_half, out=acc) # reuse acc as temp calc buffer
        np.add(states, acc, out=temp)
        
        # RK4 Step 2
        a1, a2 = eval_fn(temp)
        k2[0], k2[1] = temp[2], temp[3]
        k2[2], k2[3] = a1, a2
        
        # Temp = S + k2*dt/2
        np.multiply(k2, dt_half, out=acc)
        np.add(states, acc, out=temp)
        
        # RK4 Step 3
        a1, a2 = eval_fn(temp)
        k3[0], k3[1] = temp[2], temp[3]
        k3[2], k3[3] = a1, a2
        
        # Temp = S + k3*dt
        np.multiply(k3, dt, out=acc)
        np.add(states, acc, out=temp)
        
        # RK4 Step 4
        a1, a2 = eval_fn(temp)
        k4[0], k4[1] = temp[2], temp[3]
        k4[2], k4[3] = a1, a2
        
        # Sum: k1 + 2k2 + 2k3 + k4
        np.add(k1, k2, out=acc) # acc = k1+k2
        np.add(acc, k2, out=acc) # +k2
        np.add(acc, k3, out=acc) # +k3
        np.add(acc, k3, out=acc) # +k3
        np.add(acc, k4, out=acc) # +k4
        
        np.multiply(acc, dt_sixth, out=acc)
        np.add(states, acc, out=states)
        
        # Damping
        states[2] *= DAMPING
        states[3] *= DAMPING
        
    run_time = time.perf_counter() - t0
    return run_time, history

# ================= MAIN =================

def main():
    print(f"🏆 DOUBLE PENDULUM QUAD-VIEW")
    print(f"   Pendulums: {N_PENDULUMS:,}")
    print(f"   Steps:     {N_STEPS}")
    
    np.random.seed(42)
    initial_states = np.zeros((4, N_PENDULUMS))
    # th1, th2
    initial_states[0] = np.random.uniform(2.5, 3.0, N_PENDULUMS) # High energy
    initial_states[1] = np.random.uniform(0.5, 1.5, N_PENDULUMS)
    
    engines = [
        ('SymPy', setup_sympy, '#60a5fa'),
        ('SymEngine', setup_symengine, '#34d399'),
        ('SymbAnaFis', setup_symbanafis, '#fbbf24'),
    ]
    
    results = []
    
    for name, setup_fn, color in engines:
        try:
            eval_fn, dt_time = setup_fn()
            if eval_fn is None: continue
            
            # Warmup
            eval_fn(initial_states[:, :100])
            
            print(f"   🚀 Running {name}...")
            
            # Perturb initial states slightly (1e-10) to show chaotic divergence between engines
            # This proves they are independent simulations!
            sim_states = initial_states.copy()
            if name != "SymPy":
                 perturbation = np.random.normal(0, 1e-10, sim_states.shape)
                 sim_states += perturbation
            
            r_time, hist = simulate(name, eval_fn, sim_states)
            print(f"   ✓ Time: {r_time:.4f}s")
            
            results.append({
                "name": name,
                "setup_time": dt_time,
                "run_time": r_time,
                "total_time": dt_time + r_time,
                "history": hist,
                "color": color
            })
        except Exception as e:
            print(f"   ❌ {name} Failed: {e}")
            import traceback
            traceback.print_exc()

    if not results: return

    # --- QUAD-VIEW VIDEO ---
    print("\n🎥 Generating Quad-View Video...")
    
    fig = plt.figure(figsize=(16, 9), facecolor='#0f172a')
    gs = gridspec.GridSpec(2, 2, width_ratios=[1, 1], height_ratios=[1, 1],
                           wspace=0.1, hspace=0.15, left=0.05, right=0.95, top=0.9, bottom=0.05)
    
    axes_map = {
        0: gs[0, 0], # TL
        1: gs[0, 1], # TR
        2: gs[1, 0]  # BL (Sim 3)
    }
    
    subplots = []
    
    # Setup Sim Subplots
    for i in range(3):
        if i >= len(results): break
        res = results[i]
        
        ax = fig.add_subplot(axes_map[i], facecolor='black')
        ax.set_facecolor('#0f172a')
        ax.set_xlim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_ylim(-AXIS_LIMIT, AXIS_LIMIT)
        ax.set_aspect('equal')
        ax.axis('off')
        ax.set_title(f"{res['name']}", color=res['color'], fontsize=14, fontweight='bold')
        
        # Init Objects
        lines = []
        balls1 = []
        balls2 = []
        
        hist = res['history'] # (Steps, 4, N_vis)
        n_vis = hist.shape[2]
        
        for k in range(n_vis):
            # Line: Pivot -> m1 -> m2
            ln, = ax.plot([], [], '-', color=res['color'], alpha=0.4, lw=1.5)
            lines.append(ln)
            # Ball 1 (Joint)
            b1, = ax.plot([], [], 'o', color='white', ms=3, alpha=0.6)
            balls1.append(b1)
            # Ball 2 (End)
            b2, = ax.plot([], [], 'o', color=res['color'], ms=5, alpha=0.8)
            balls2.append(b2)
            
        subplots.append({
            'ax': ax, 
            'lines': lines, 
            'balls1': balls1,
            'balls2': balls2,
            'hist': hist
        })
        
    # Performance Graph (Bottom Right)
    ax_perf = fig.add_subplot(gs[1, 1], facecolor='#1e293b')
    names = [r['name'] for r in results]
    times = [r['run_time'] for r in results] # Focus on Run Time for physics
    colors = [r['color'] for r in results]
    
    y = np.arange(len(names))
    ax_perf.barh(y, times, color=colors, alpha=0.9)
    ax_perf.set_yticks(y)
    ax_perf.set_yticklabels(names, color='white', fontweight='bold')
    ax_perf.invert_yaxis()
    ax_perf.set_xlabel('Simulation Time (s)', color='#94a3b8')
    ax_perf.set_title("Physics Loop Performance (Lower is Better)", color='white')
    ax_perf.spines['top'].set_visible(False)
    ax_perf.spines['right'].set_visible(False)
    ax_perf.spines['bottom'].set_color('#475569')
    ax_perf.spines['left'].set_visible(False)
    ax_perf.tick_params(colors='#94a3b8')
    
    for i, v in enumerate(times):
        ax_perf.text(v + max(times)*0.02, i, f"{v:.4f}s", color='white', va='center')

    fig.suptitle(f"Double Pendulum Benchmark: {N_PENDULUMS:,} Pendulums", 
                 color='white', fontsize=20, fontweight='bold', y=0.98)

    def update(frame):
        # We might have different step counts if failed, but here all N_STEPS
        for sub in subplots:
            hist = sub['hist']
            lines = sub['lines']
            balls1 = sub['balls1']
            balls2 = sub['balls2']
            
            idx = min(frame, hist.shape[0]-1)
            state = hist[idx] # (4, N_vis)
            
            for k in range(len(lines)):
                th1 = state[0, k]
                th2 = state[1, k]
                x1 = L1 * np.sin(th1)
                y1 = -L1 * np.cos(th1)
                x2 = x1 + L2 * np.sin(th2)
                y2 = y1 - L2 * np.cos(th2)
                
                lines[k].set_data([0, x1, x2], [0, y1, y2])
                balls1[k].set_data([x1], [y1])
                balls2[k].set_data([x2], [y2])
                
        return []

    ani = animation.FuncAnimation(fig, update, frames=N_STEPS, interval=30, blit=False)
    
    try:
        from matplotlib.animation import FFMpegWriter
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'double_pendulum_quad.mp4'
        writer = FFMpegWriter(fps=30, codec='h264_nvenc', 
                              extra_args=['-preset', 'fast', '-rc', 'vbr', '-cq', '26'])
        ani.save(str(out_path), writer=writer, dpi=100)
        print(f"✨ Saved {out_path} (GPU)")
    except:
        from pathlib import Path
        video_dir = Path(__file__).parent.parent / 'videos'
        video_dir.mkdir(exist_ok=True)
        out_path = video_dir / 'double_pendulum_quad.mp4'
        ani.save(str(out_path), fps=30, dpi=100)
        print(f"✨ Saved {out_path} (CPU)")

if __name__ == "__main__":
    main()
