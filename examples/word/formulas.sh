#!/bin/bash
# Generate complex math/chemistry/physics formula test document
# Usage: ./formulas.sh [officecli path]

set -e
CLI="${1:-officecli}"
OUT="$(dirname "$0")/formulas.docx"

rm -f "$OUT"
$CLI create "$OUT"

# ==================== Title ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="Complex Math/Chemistry/Physics Formula Collection" style=Heading1 alignment=center

# ==================== I. Algebra ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="I. Algebra" style=Heading2

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="1. Quadratic Formula:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=x = \frac{-b \pm \sqrt{b^{2} - 4ac}}{2a}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="2. Binomial Theorem:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=(a+b)^{n} = \sum_{k=0}^{n} \binom{n}{k} a^{n-k} b^{k}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="3. Euler's Identity:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=e^{i\pi} + 1 = 0'

# ==================== II. Calculus ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="II. Calculus" style=Heading2

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="4. Limit Definition of Derivative:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=f^{\prime}(x) = \lim_{\Delta x \rightarrow 0} \frac{f(x + \Delta x) - f(x)}{\Delta x}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="5. Gaussian Integral:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\int_{-\infty}^{+\infty} e^{-x^{2}} dx = \sqrt{\pi}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="6. Taylor Series Expansion:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=f(x) = \sum_{n=0}^{\infty} \frac{f^{(n)}(a)}{n!} (x-a)^{n}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="7. Newton-Leibniz Formula:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\int_{a}^{b} f(x) dx = F(b) - F(a)'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="8. Triple Integral (Spherical Coordinates):"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\iiint_{V} f(r, \theta, \phi) r^{2} \sin\theta \, dr \, d\theta \, d\phi'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="9. Fourier Transform:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\hat{f}(\xi) = \int_{-\infty}^{+\infty} f(x) e^{-2\pi i x \xi} dx'

# ==================== III. Linear Algebra ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="III. Linear Algebra" style=Heading2

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="10. Matrix Characteristic Equation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\det(A - \lambda I) = 0'

# ==================== IV. Probability and Statistics ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="IV. Probability and Statistics" style=Heading2

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="11. Bayes' Theorem:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=P(A|B) = \frac{P(B|A) \cdot P(A)}{P(B)}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="12. Normal Distribution PDF:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=f(x) = \frac{1}{\sigma \sqrt{2\pi}} e^{-\frac{(x-\mu)^{2}}{2\sigma^{2}}}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="13. Variance Formula:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\sigma^{2} = \frac{1}{N} \sum_{i=1}^{N} (x_{i} - \mu)^{2}'

# ==================== V. Number Theory and Series ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="V. Number Theory and Series" style=Heading2

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="14. Riemann Zeta Function:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\zeta(s) = \sum_{n=1}^{\infty} \frac{1}{n^{s}}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="15. Stirling's Approximation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=n! \approx \sqrt{2\pi n} \left(\frac{n}{e}\right)^{n}'

# ==================== VI. Chemistry ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="VI. Chemistry" style=Heading2

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="16. Copper Sulfate Crystal Dissolution:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=CuSO_{4} \cdot 5H_{2}O \rightarrow Cu^{2+} + SO_{4}^{2-} + 5H_{2}O'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="17. Thermochemical Equation (Methane Combustion):"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=CH_{4}(g) + 2O_{2}(g) \rightarrow CO_{2}(g) + 2H_{2}O(l) \quad \Delta H = -890.3 \, kJ/mol'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="18. Chemical Equilibrium Constant Expression:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=K_{eq} = \frac{[C]^{c} [D]^{d}}{[A]^{a} [B]^{b}}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="19. Esterification Reaction (Reversible):"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=CH_{3}COOH + C_{2}H_{5}OH \rightleftharpoons CH_{3}COOC_{2}H_{5} + H_{2}O'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="20. Henderson-Hasselbalch Equation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=pH = pK_{a} + \log \frac{[A^{-}]}{[HA]}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="21. Van der Waals Equation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\left(P + \frac{a n^{2}}{V^{2}}\right)(V - nb) = nRT'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="22. Arrhenius Equation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=k = A e^{-\frac{E_{a}}{RT}}'

# ==================== VII. Physics ====================
$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="VII. Physics" style=Heading2

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="23. Maxwell's Equations (Differential Form):"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\nabla \cdot E = \frac{\rho}{\epsilon_{0}}'
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\nabla \cdot B = 0'
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\nabla \times E = -\frac{\partial B}{\partial t}'
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\nabla \times B = \mu_{0} J + \mu_{0} \epsilon_{0} \frac{\partial E}{\partial t}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="24. Einstein Field Equations:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=R_{\mu\nu} - \frac{1}{2} R g_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^{4}} T_{\mu\nu}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="25. Schrodinger Equation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=i\hbar \frac{\partial}{\partial t} \Psi(r, t) = \hat{H} \Psi(r, t)'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="26. Dirac Equation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=(i\gamma^{\mu} \partial_{\mu} - m) \psi = 0'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="27. Euler-Lagrange Equation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\frac{d}{dt} \frac{\partial L}{\partial \dot{q}_{i}} - \frac{\partial L}{\partial q_{i}} = 0'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="28. Heisenberg Uncertainty Principle:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=\Delta x \cdot \Delta p \geq \frac{\hbar}{2}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="29. Planck's Black-Body Radiation Formula:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=B(\nu, T) = \frac{2h\nu^{3}}{c^{2}} \cdot \frac{1}{e^{\frac{h\nu}{k_{B} T}} - 1}'

$CLI add "$OUT" --parent '/body' --type-name paragraph --properties text="30. Lorentz Transformation:"
$CLI add "$OUT" --parent '/body' --type-name equation --properties 'formula=t^{\prime} = \gamma \left(t - \frac{vx}{c^{2}}\right), \quad \gamma = \frac{1}{\sqrt{1 - \frac{v^{2}}{c^{2}}}}'

echo "Generated: $OUT"