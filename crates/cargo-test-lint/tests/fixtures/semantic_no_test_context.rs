
fn helper() {
    let code = r#"
import pytest
from myapp import Calculator

def test_addition():
    calc = Calculator()
    result = calc.add(2, 3)
    assert result == 5
"#;
}
