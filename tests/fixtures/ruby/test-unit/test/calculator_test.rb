require 'test/unit'
require_relative '../lib/calculator'

class CalculatorTest < Test::Unit::TestCase
  def test_add
    calc = Calculator.new
    assert_equal(5, calc.add(2, 3))
  end
end
