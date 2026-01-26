class DSL
  def configure(obj, &block)
    # Missing METAPROGRAMMING comment - should fail
    obj.instance_eval(&block)
  end
end
