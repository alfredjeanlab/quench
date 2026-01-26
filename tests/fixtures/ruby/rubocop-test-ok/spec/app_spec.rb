# rubocop:disable Style/Documentation
RSpec.describe App do
  it 'runs' do
    expect(App.new.run).to be_nil
  end
end
# rubocop:enable Style/Documentation
