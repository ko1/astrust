require_relative '../../lib/astrustgen'
class CalcNodeDef < ASTrustGen::NodeDef
  class Node < ASTrustGen::NodeDef::Node
    def result_type = 'i64'
  end
end
ASTrustGen.start(ARGV, node_def_class: CalcNodeDef)
