require_relative 'boot'

module MyApp
  class Application < Rails::Application
    config.load_defaults 7.0
  end
end
