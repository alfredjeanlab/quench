# rubocop:disable Style/Documentation, Metrics/MethodLength
class Handler
  def handle(input)
    result = []
    result << input.upcase
    result << input.downcase
    result << input.reverse
    result
  end
end
# rubocop:enable Style/Documentation, Metrics/MethodLength
