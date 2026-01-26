module Macro
  def self.define_method_on(klass, name, &block)
    # Missing METAPROGRAMMING comment - should fail
    klass.class_eval do
      define_method(name, &block)
    end
  end
end
