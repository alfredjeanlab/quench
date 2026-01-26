RSpec.describe 'Dynamic code' do
  it 'executes code via eval' do
    # eval in test code is allowed
    result = eval('1 + 2')
    expect(result).to eq(3)
  end
end
