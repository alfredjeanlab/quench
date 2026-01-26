require 'example'

RSpec.describe Example do
  it 'returns greeting' do
    expect(Example.hello).to eq("Hello, World!")
  end
end
