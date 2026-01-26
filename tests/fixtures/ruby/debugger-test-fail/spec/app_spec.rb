RSpec.describe App do
  it 'runs' do
    binding.pry
    expect(App.new.run).to be_nil
  end
end
