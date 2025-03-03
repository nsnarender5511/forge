from rest_framework import serializers
from .models import Todo

class TodoSerializer(serializers.ModelSerializer):
    """Serializer for Todo model"""
    
    user = serializers.ReadOnlyField(source='user.username')
    
    class Meta:
        model = Todo
        fields = [
            'id', 
            'title', 
            'description', 
            'status', 
            'priority', 
            'due_date',
            'user',
            'created_at', 
            'updated_at'
        ]
        read_only_fields = ['id', 'created_at', 'updated_at', 'user']
        
class TodoCreateSerializer(serializers.ModelSerializer):
    """Serializer for creating Todo items"""
    
    class Meta:
        model = Todo
        fields = [
            'title', 
            'description', 
            'status', 
            'priority', 
            'due_date',
        ]
        
class TodoListSerializer(serializers.ModelSerializer):
    """Serializer optimized for listing Todo items"""
    
    class Meta:
        model = Todo
        fields = [
            'id', 
            'title', 
            'status', 
            'priority', 
            'due_date',
        ]