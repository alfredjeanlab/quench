require_relative '../../lib/greeter'

Given('a greeter') do
  @greeter = Greeter.new
end

When('I greet {string}') do |name|
  @result = @greeter.greet(name)
end

Then('the result should be {string}') do |expected|
  expect(@result).to eq(expected)
end
