RSpec.describe 'Debugging' do
  it 'allows debugger with config' do
    # With in_tests = "allow", this passes
    binding.pry
    expect(true).to be true
  end
end
